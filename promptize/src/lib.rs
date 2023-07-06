use proc_macro::TokenStream;
use quote::quote;
use syn::{
    parse_macro_input, DeriveInput
};


/// Allows fields of a struct to be marked as Chunkable as to denote that they are able to be
/// broken up into chunks when being fed to an LLM to fit inside of a context window.
#[proc_macro_derive(Promptize, attributes(chunkable))]
pub fn promptize(input: TokenStream) -> TokenStream {
    let input_ast = parse_macro_input!(input as DeriveInput);
    println!("ast: {:#?}", input_ast);

    let name = &input_ast.ident;
    let builder_name = format!("{}Builder", name); 
    let builder_ident = syn::Ident::new(&builder_name, name.span());

    let fields = if let syn::Data::Struct(syn::DataStruct { 
        fields: syn::Fields::Named(syn::FieldsNamed { 
            ref named, 
            ..
        }), 
        ..
    }) = input_ast.data {
        named 
    } else {
        panic!("Only implemented for Struct");
    };

    let (chunkable_fields, _unchunkable_fields) = split_fields(fields.clone()); 

    if chunkable_fields.len() > 1 {
        let error = syn::Error::new(name.span(), "chunkable attribute is only supported on one field at a time");
        return error.to_compile_error().into();
    }

    let chunk_field = chunkable_fields.first().unwrap();
    let cf_name = &chunk_field.ident;
    let cf_type = &chunk_field.ty;

    // Ensure chunkable field is of type string
    match cf_type {
        syn::Type::Path(p) => {
            let is_string = p.path.segments.first().unwrap().ident == "String";
            
            if !is_string {
                let error = syn::Error::new(name.span(), "Only String type supported for chunkable fields");
                return error.to_compile_error().into();
            }
        },
        _ => panic!("only syn::Type::Path supported on Field Type")
    };

    let chunk_field = quote! {
        #cf_name
    };

    let has_user = &fields.iter().any(|f| { f.ident.clone().unwrap() == "user_prompt" });
    let has_system = &fields.iter().any(|f| { f.ident.clone().unwrap() == "system_prompt" });

    if !has_user || !has_system {
        let error = syn::Error::new(name.span(), "user and system fields are required to be defined on struct");
        return error.to_compile_error().into();
    }

    fn is_optional(field: &syn::Field) -> bool {
        if let syn::Type::Path(t_path) = &field.ty {
            let segments = &t_path.path.segments;
            if segments.len() == 1 && segments[0].ident == "Option" {
                return true
            }
            return false
        } else {
            panic!("unsupported type path")
        }
    }

    let template_fields = fields.iter().map(|f| {
        let name = &f.ident;
        let ty = &f.ty;

        if is_optional(&f) {
            return quote! {
                #name: #ty
            };
        }

        return quote! {
            #name: std::option::Option<#ty>
        };
    });

    let fields_empty = fields.iter().map(|f| {
        let name = &f.ident;
        quote! {
            #name: None
        }
    });

    let builder_methods = fields.iter().map(|f| {
        let name = &f.ident;
        let ty = &f.ty;

        if is_optional(&f) {
            // extract root type
            let option_type = get_option_type(&f);
            return quote! {
                pub fn #name(&mut self, #name: #option_type) -> &mut Self {
                    self.#name = Some(#name);
                    self
                }
            };
        }

        return quote! {
            pub fn #name(&mut self, #name: #ty) -> &mut Self {
                self.#name = Some(#name);
                self
            }
        };
    });

    let expanded = quote! {
        #[derive(serde::Serialize, Clone)]
        struct #builder_ident {
            #(#template_fields),*
        }
        
        impl #builder_ident {
            #(#builder_methods)*

            pub fn build_prompt(
                &self, 
                model: &str, 
                token_limit: i32,
                chunkable_token_limit: i32
            ) -> Result<
                std::vec::Vec<std::vec::Vec<tiktoken_rs::ChatCompletionRequestMessage>>, 
                std::boxed::Box<dyn std::error::Error>
            > {
                let prompt_string = serde_json::to_string(&self)?;
                let total_prompt_tokens: i32 = get_prompt_tokens(model, &prompt_string)?.try_into()?;

                if total_prompt_tokens > token_limit {
                    let chunk_field = self.#chunk_field.clone().ok_or(concat!(stringify!(#name), " is not set"))?;
                    let chunkable_field_tokens: i32 = get_prompt_tokens(model, &chunk_field)?.try_into()?;
                    
                    // this represents the tokens left after non chunkable fields are removed
                    // since non chunkable fields cannot be changed, this is our "real" limit
                    let chunkable_tokens_remaining = token_limit - (total_prompt_tokens - chunkable_field_tokens);
                    
                    // 8000 - (10000 - 8000) = 6000
                    // 8000 - (10000 - 1000) = -1000
                    // 8000 - (10000 - 3000) = 1000
                    
                    // we need to set a reasonable limit for "real_token_limit"
                    // ex: if we only have 1000 tokens but the chunkable field is 20000
                    // we don't want to call the API 20 times
                    if chunkable_tokens_remaining < chunkable_token_limit {
                        return Err("chunkable_tokens_remaining is less than chunkable token limit".into());
                    }

                    let chunk_size_tokens = get_chunk_size_tokens(chunkable_field_tokens, chunkable_tokens_remaining);

                    let chunk_ratio = chunk_size_tokens as f64 / chunkable_field_tokens as f64;
                    let total_chars = prompt_string.chars().collect::<Vec<char>>().len();
                    let chunk_size_chars:i32 = (chunk_ratio * total_chars as f64).ceil() as i32;

                    let string_chunks = chunk_string(prompt_string, chunk_size_chars);

                    let prompts: Vec<Vec<tiktoken_rs::ChatCompletionRequestMessage>> = string_chunks
                        .iter()
                        .map(|c| {
                            let mut prompt = vec![];
                            let system = tiktoken_rs::ChatCompletionRequestMessage {
                                role: "system".to_string(),
                                content: self.system_prompt.clone().unwrap(),
                                name: None
                            };

                            let user = tiktoken_rs::ChatCompletionRequestMessage {
                                role: "user".to_string(),
                                content: c.clone(),
                                name: None
                            };

                            prompt.push(system);
                            prompt.push(user);
                            prompt
                        })
                        .collect();

                    return Ok(prompts);
                }

                // TODO: Maybe do this right?
                let system = tiktoken_rs::ChatCompletionRequestMessage {
                    role: "system".to_string(),
                    content: self.system_prompt.clone().unwrap(),
                    name: None
                };

                let user = tiktoken_rs::ChatCompletionRequestMessage {
                    role: "user".to_string(),
                    content: self.user_prompt.clone().unwrap(),
                    name: None
                };

                Ok(vec![vec![system, user]])
            }
        }

        impl #name {
            fn builder() -> #builder_ident {
                #builder_ident {
                    #(#fields_empty,)*
                }
            }
        }

        /// Gets the optimal chunk size in Tokens
        fn get_chunk_size_tokens(total: i32, limit: i32) -> i32 {
            let num_chunks = (total as f64 / limit as f64).ceil() as i32;
            let base_chunk_size = total / num_chunks;

            let mut chunk_size = base_chunk_size;
            let mut num_chunks = num_chunks;

            while chunk_size > limit {
                num_chunks += 1; 
                chunk_size = total / num_chunks;
            }
            chunk_size
        }
 
        /// Chunks up a string based on chunk_size which is number of chars not tokens
        fn chunk_string(prompt: String, chunk_size: i32) -> std::vec::Vec<String> {
            let chunks = prompt
                .chars()
                .collect::<std::vec::Vec<char>>()
                .chunks(chunk_size as usize)
                .map(|c| c.iter().collect::<String>())
                .collect::<std::vec::Vec<String>>();

            chunks
        }

        fn get_prompt_tokens(model: &str, prompt: &str) -> Result<usize, std::boxed::Box<dyn std::error::Error>> {
            let bpe = tiktoken_rs::get_bpe_from_model(model)?;
            let prompt_tokens = bpe.encode_with_special_tokens(prompt).len();
            Ok(prompt_tokens)
        }
        
    };

    proc_macro::TokenStream::from(expanded)
}

fn split_fields(fields: syn::punctuated::Punctuated<syn::Field, syn::token::Comma>) -> (Vec<syn::Field>, Vec<syn::Field>) {
    let (chunkable, not): (Vec<_>, Vec<_>) = fields
        .into_iter()
        .partition(|field| {
            field.attrs.iter().any(|attr| {
                match attr.meta.clone() {
                    syn::Meta::Path(path) => path.is_ident("chunkable"),
                    _ => false,
                }
            })
        });

    return (chunkable, not);
}

fn get_option_type(field: &syn::Field) -> syn::Ident {
    match &field.ty {
        syn::Type::Path(t_path) => {
            let segments = &t_path.path.segments;
            match &segments[0].arguments {
                syn::PathArguments::AngleBracketed(af) => {
                    let first_arg = af.args.first().unwrap();
                    match first_arg {
                        syn::GenericArgument::Type(arg) => {
                            match arg {
                                syn::Type::Path(p) => {
                                    return p.path.get_ident().unwrap().to_owned();
                                },
                                _ => unimplemented!("Arg not of Type::Path")
                            }
                        },
                        _ => unimplemented!("Path Argument not GenericArgument::Type")
                    }
                },
                _ => unimplemented!("PathArgument not AngleBracketed")
            }
        },
        _ => unimplemented!("Type not a path")
    }
}
