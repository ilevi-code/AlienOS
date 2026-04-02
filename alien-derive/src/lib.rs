use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields};

#[proc_macro_derive(Pod)]
pub fn derive_my_marker(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let name = input.ident;

    let field_types = match input.data {
        Data::Struct(data_struct) => match data_struct.fields {
            Fields::Named(fields) => fields
                .named
                .iter()
                .map(|f| f.ty.clone())
                .collect::<Vec<_>>(),
            Fields::Unnamed(fields) => fields
                .unnamed
                .iter()
                .map(|f| f.ty.clone())
                .collect::<Vec<_>>(),
            Fields::Unit => vec![],
        },
        _ => {
            return TokenStream::from(
                syn::Error::new(name.span(), "Only structs can derive `Pod`").to_compile_error(),
            )
        }
    };

    let expanded = quote! {

        impl alien_traits::Pod for #name
        where
            #( #field_types: Pod ),*
        {}
    };

    TokenStream::from(expanded)
}
