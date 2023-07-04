use proc_macro::TokenStream;
use quote::quote;
use syn::{
    parse_macro_input, DeriveInput
};

/// Allows fields of a struct to be marked as Chunkable as to denote that they are able to be
/// broken up into chunks when being fed to an LLM to fit inside of a context window.
#[proc_macro_derive(Chunkable, attributes(chunkable))]
pub fn chunk_it_up(input: TokenStream) -> TokenStream {
    // Parse the input tokens into a syntax tree.
    let input_ast = parse_macro_input!(input as DeriveInput);
    println!("{:#?}", input_ast);

    let struct_name = input_ast.ident;
    let input_struct = match_struct(input_ast.data);

    let (chunkable_fields, unchunkable_fields) = split_fields(input_struct); 

    let unchunk_fields = unchunkable_fields.iter().map(|f| {
        let name = &f.ident;

        quote! {
            #name
        }
    });

    if chunkable_fields.len() > 1 {
        panic!("chunkable attribute is only supported on one field at a time")
    }

    let chunk_field = chunkable_fields.first().unwrap();
    let cf_name = &chunk_field.ident;

    let chunk_field = quote! {
        #cf_name
    };

    let expanded = quote! {
        impl #struct_name {
            fn chunk_it(&self) -> Vec<Self> {
                let mut foo = vec![];
                let chunk_size = 2;

                for i in 0..chunk_size {
                    foo.push(#struct_name {
                        #(#unchunk_fields: self.#unchunk_fields.clone(),)*
                        #chunk_field: self.#chunk_field.clone()
                    })
                }
                foo
            }
        }
    };

    // Hand the output tokens back to the compiler.
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

fn split_fields(chunkable_struct: syn::DataStruct) -> (Vec<syn::Field>, Vec<syn::Field>) {
    let (chunkable, not): (Vec<_>, Vec<_>) = chunkable_struct.fields
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
