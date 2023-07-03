use proc_macro::TokenStream;
use quote::quote;
use syn::{
    parse_macro_input, DeriveInput
};

#[proc_macro_derive(Builder)]
pub fn derive(input: TokenStream) -> TokenStream {
    let input_ast = parse_macro_input!(input as DeriveInput);
    println!("{:#?}", input_ast);
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

    let template_fields = fields.iter().map(|f| {
        let name = &f.ident;
        let ty = &f.ty;

        quote! {
            #name: std::option::Option<#ty>
        }
    });

    let fields_empty = fields.iter().map(|f| {
        let name = &f.ident;
        quote! {
            #name: None
        }
    });

    let methods = fields.iter().map(|f| {
        let name = &f.ident;
        let ty = &f.ty;

        quote! {
            pub fn #name(&mut self, #name: #ty) -> &mut Self {
                self.#name = Some(#name);
                self
            }

        }
    });

    let build_fields = fields.iter().map(|f| {
        let name = &f.ident;
        quote! {
            #name: self.#name.clone().ok_or(concat!(stringify!(#name), " is not set"))?
        }
    });

    let expanded = quote! {
        struct #builder_ident {
            #(#template_fields),*
        }
        
        impl #builder_ident {
            #(#methods)*

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
