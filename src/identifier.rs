use crate::{constants::*, helper::*, msi_tables::FieldInformation};
use proc_macro2::TokenStream;
use quote::quote;

pub fn generate_identifier_tokens(
    target_name: &str,
    primary_identifier: &FieldInformation,
) -> TokenStream {
    let identifier_impl_tokens = generate_identifier_definition(target_name);

    // If the primary identifier requires a generator, create that now.
    let identifier_generator_definition_tokens = if let Some(identifier_options) =
        &primary_identifier.identifier_options
        && identifier_options.generated
    {
        generate_identifier_generator_definition(target_name)
    } else {
        Default::default()
    };

    quote! {
        #identifier_impl_tokens
        #identifier_generator_definition_tokens
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
        #[derive(Clone, Debug, Default, PartialEq, derive_more::Display)]
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

fn generate_identifier_generator_definition(target_name: &str) -> TokenStream {
    let identifier_ident = identifier_from_name(target_name);
    let identifier_generator_struct_ident = identifier_generator_from_name(target_name);
    let identifier_prefix = target_name.to_uppercase();
    quote! {
        #[derive(Debug, Clone, Default, PartialEq)]
        pub(crate) struct #identifier_generator_struct_ident {
            count: usize,
            // A reference to a vec of all used Identifiers that should not be generated again.
            // These are all identifiers that inhabit a primary_key column.
            used: std::rc::Rc<std::cell::RefCell<Vec<Identifier>>>,
        }

        impl IdentifierGenerator for #identifier_generator_struct_ident {
            type IdentifierType = #identifier_ident;

            fn id_prefix(&self) -> &str {
                #identifier_prefix
            }

            fn used(&self) -> &std::rc::Rc<std::cell::RefCell<Vec<Identifier>>> {
                &self.used
            }

            fn count(&self) -> usize {
                self.count
            }

            fn count_mut(&mut self) -> &mut usize {
                &mut self.count
            }
        }

        impl From<std::rc::Rc<std::cell::RefCell<Vec<Identifier>>>> for #identifier_generator_struct_ident {
            fn from(value: std::rc::Rc<std::cell::RefCell<Vec<Identifier>>>) -> Self {
                let count = value.borrow().len();
                Self {
                    used: value,
                    count: 0,
                }
            }
        }
    }
}
