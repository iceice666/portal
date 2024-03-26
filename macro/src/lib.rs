use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput};

#[proc_macro_attribute]
pub fn derive_conversion_with_u8(_attr: TokenStream, input: TokenStream) -> TokenStream {
    let mut extended = TokenStream2::from(input.clone());

    let parsed_input = parse_macro_input!(input as DeriveInput);
    let name = &parsed_input.ident;

    let fields = match parsed_input.data {
        Data::Enum(ref data) => data.variants.iter().map(|v| &v.ident),
        _ => panic!("Only enums are supported"),
    };

    let mut convert_block = TokenStream2::new();
    for (i, field) in fields.enumerate() {
        let i = i as u8;
        convert_block.extend(quote! {
            #i => Ok(#name::#field),
        });
    }
    convert_block.extend(quote! {
        _ => Err(anyhow::anyhow!("Invalid value for {}", stringify!(#name))),
    });

    extended.extend(quote! {
        impl std::convert::TryFrom<u8> for #name {
            type Error = ::anyhow::Error;

            fn try_from(value: u8) -> Result<#name, Self::Error> {
                match value {
                    #convert_block
                }
            }
        }

        impl From<#name> for u8 {
            fn from(value: #name) -> Self {
                value.try_into().unwrap()
            }
        }
    });

    TokenStream::from(extended)
}
