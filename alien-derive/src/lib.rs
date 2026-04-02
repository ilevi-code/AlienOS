use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, spanned::Spanned, Data, DeriveInput, Fields};

#[proc_macro_derive(Pod)]
pub fn derive_pod(input: TokenStream) -> TokenStream {
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
            );
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

#[proc_macro_derive(IntEnum, attributes(default))]
pub fn derive_int_enum(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let name = input.ident;

    let mut variant_idents = vec![];
    let mut variant_discriminants = vec![];
    let mut default_variant = None;

    match input.data {
        Data::Enum(data_enum) => {
            for variant in data_enum.variants {
                match &variant.fields {
                    syn::Fields::Unit => (),
                    _ => {
                        return TokenStream::from(
                            syn::Error::new(
                                variant.fields.span(),
                                "IntEnum's variants must be unit-types",
                            )
                            .to_compile_error(),
                        );
                    }
                }
                let is_default = variant
                    .attrs
                    .iter()
                    .any(|attr| attr.path().is_ident("default"));

                if is_default {
                    default_variant = Some(variant.ident);
                } else {
                    variant_idents.push(variant.ident);
                    variant_discriminants.push(variant.discriminant.unwrap().1);
                }
            }
        }
        _ => {
            return TokenStream::from(
                syn::Error::new(name.span(), "Only enums can derive `IntEnum`").to_compile_error(),
            );
        }
    };

    if default_variant.is_none() {
        return TokenStream::from(
            syn::Error::new(name.span(), "missing #[default] variant").to_compile_error(),
        );
    }

    let expanded = quote! {
        impl core::convert::From<u32> for #name
        {
            fn from(value: u32) -> Self {
                 match value {
                     #( #variant_discriminants => Self::#variant_idents,)*
                     _ => Self::#default_variant,
                 }
            }
        }
    };

    TokenStream::from(expanded)
}
