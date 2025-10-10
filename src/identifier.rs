use crate::{constants::*, helper::*, msi_tables::FieldInformation};
use proc_macro2::TokenStream;
use quote::quote;

pub fn generate_identifier_tokens(
    target_name: &str,
    primary_identifier: &FieldInformation,
) -> TokenStream {
    let identifier_impl_tokens = generate_identifier_definition(target_name);
    quote! {
        #identifier_impl_tokens
    }
}

fn generate_identifier_definition(target_name: &str) -> TokenStream {
    let new_identifier_ident = identifier_from_name(target_name);

    let identifier_comment = &format!(
        "This is a simple wrapper around `Identifier` for the `{target_name}{TABLE_SUFFIX}`. \
        Used to ensure that identifiers for the `{target_name}{TABLE_SUFFIX}` are only used in valid locations."
    );
    quote! {
        #[doc = #identifier_comment]
        #[derive(Clone, Debug, Default, PartialEq, derive_more::Display, whimsi_macros::IdentifierToValue)]
        pub struct #new_identifier_ident(Identifier);

        impl ToIdentifier for #new_identifier_ident {
            fn to_identifier(&self) -> Identifier {
                self.0
            }
        }

        impl std::str::FromStr for #new_identifier_ident {
            type Err = anyhow::Error;

            fn from_str(s: &str) -> anyhow::Result<Self> {
                Ok(Self(Identifier::from_str(s)?))
            }
        }
    }
}
