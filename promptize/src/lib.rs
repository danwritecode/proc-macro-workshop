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

    let (chunkable_fields, unchunkable_fields) = split_fields(fields.clone()); 

    let unchunk_fields = unchunkable_fields.iter().map(|f| {
        let name = &f.ident;

        quote! {
            #name
        }
    });

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

    let has_user = &fields.iter().any(|f| { f.ident.clone().unwrap() == "user" });
    let has_system = &fields.iter().any(|f| { f.ident.clone().unwrap() == "system" });

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

    let build_fields = fields.iter().map(|f| {
        let name = &f.ident;
        if is_optional(&f) {
            return quote! {
                #name: self.#name.clone()
            };
        }

        return quote! {
            #name: self.#name.clone().ok_or(concat!(stringify!(#name), " is not set"))?
        };
    });

    let expanded = quote! {
        struct #builder_ident {
            #(#template_fields),*
        }
        
        impl #builder_ident {
            #(#builder_methods)*

            pub fn build_prompt(&self) -> Result<Vec<#name>, Box<dyn std::error::Error>> {
                Ok(vec![#name {
                    #(#build_fields,)*
                }])
            }
        }

        impl #name {
            fn builder() -> #builder_ident {
                #builder_ident {
                    #(#fields_empty,)*
                }
            }
        }
    };

    proc_macro::TokenStream::from(expanded)
}

fn match_struct(data: syn::Data) -> syn::DataStruct {
    match data {
        syn::Data::Struct(s) => {
            return s;
        },
        _ => {panic!("Derive Chunkable only supported for Structs")}
    };
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
