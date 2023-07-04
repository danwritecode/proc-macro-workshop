use proc_macro::TokenStream;
use quote::quote;
use syn::{
    parse_macro_input, DeriveInput
};

#[proc_macro_derive(Builder)]
pub fn derive(input: TokenStream) -> TokenStream {
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

            pub fn build(&self) -> Result<#name, Box<dyn std::error::Error>> {
                Ok(#name {
                    #(#build_fields,)*
                })
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
