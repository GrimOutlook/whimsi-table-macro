use darling::{FromDeriveInput, FromField};
use debug_print::debug_println;
use itertools::Itertools;
use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, TokenStreamExt, quote};
use syn::{DeriveInput, Field, Ident, punctuated::Punctuated, spanned::Spanned};

use crate::constants::*;
use crate::helper;

#[derive(FromDeriveInput, Clone)]
#[darling(attributes(msitable))]
struct NewTableDef {
    // Name of the defining struct
    ident: syn::Ident,
    data: darling::ast::Data<(), FieldInformation>,

    name: String,
}

#[derive(FromField, Clone)]
#[darling(attributes(msitable))]
struct FieldInformation {
    // Field name
    ident: Option<syn::Ident>,
    // Type of the field
    ty: syn::Type,

    // Denotes if the given field corresponds to a primary key in the table.
    #[darling(default)]
    primary_key: bool,

    // Denotes if the given field is an identifier.
    #[darling(default, rename = "identifier")]
    identifier_options: Option<IdentifierInformation>,
}

#[derive(darling::FromMeta, FromField, Clone)]
struct IdentifierInformation {
    // Denotes if the given identifier should have a generator created for it.
    #[darling(default)]
    generated: bool,
}

pub fn gen_tables_impl(input: TokenStream) -> TokenStream {
    let input = syn::parse2::<syn::DeriveInput>(input).unwrap();
    let new_table = NewTableDef::from_derive_input(&input).expect("Failed to parse derive input");

    debug_println!("Base name recieved: {}", new_table.name);
    let target_name = helper::capitalize(&new_table.name);

    let struct_data = new_table.data.take_struct().unwrap();
    debug_println!(
        "Fields: {:?}",
        struct_data
            .fields
            .iter()
            .map(|f| f.ident.to_token_stream().to_string())
            .collect::<Vec<_>>()
            .join(", ")
    );
    for field in struct_data.clone().fields {
        let mut debug_msg = format!(
            "Ident: [{:?}] - Primary: [{:?}]",
            field.ident, field.primary_key
        );
        if let Some(id) = field.identifier_options {
            debug_println!("Generated: {}", id.generated);
        }
    }

    // The output of the macro will start out empty and we will add tokens as we parse the data
    // provided.
    let mut output_tokens = quote! {
        use whimsi_lib::types::column::identifier::Identifier;
        use whimsi_lib::types::column::identifier::ToIdentifier;
        use whimsi_lib::types::helpers::id_generator::IdentifierGenerator;
        use whimsi_msi::types::helpers::to_msi_value::ToMsiValue;
    };

    // Create the table-specific identifier if one should be made. These are made when a table has
    // a column with a type that implements `ToIdentifier` and the column is not marked as a
    // foreign key.
    let primary_identifier = struct_data
        .clone()
        .fields
        .into_iter()
        .filter(|f| f.primary_key && f.identifier_options.is_some())
        .at_most_one()
        .unwrap_or_else(|_| {
            panic!(
                "More than one primary identifier found in [{}] defintion. This is not supported.",
                new_table.ident
            )
        });

    if let Some(primary_identifier) = primary_identifier {
        let primary_identifier_tokens =
            generate_identifier_definition(&target_name, &primary_identifier);

        // If the primary identifier requires a generator, create that now.
        let generator_tokens = if let Some(identifier_options) =
            primary_identifier.identifier_options
            && identifier_options.generated
        {
            generate_identifier_generator_definition(&target_name, primary_identifier.ident.span())
        } else {
            TokenStream::default()
        };
        output_tokens = quote! {
            #output_tokens
            #primary_identifier_tokens
            #generator_tokens
        };
    }

    debug_println!("Macro output: \n{}", output_tokens.to_string());
    output_tokens
}

fn generate_identifier_definition(
    target_name: &str,
    primary_identifier: &FieldInformation,
) -> TokenStream {
    let name = primary_identifier
        .ident
        .clone()
        .unwrap_or_else(|| panic!("Identifier for target [{}] was None", target_name));
    debug_println!(
        "Primary identifier field for struct [{}]: {}",
        target_name,
        name
    );

    let new_identifier_ident = Ident::new(
        &format!("{target_name}{IDENTIFIER_SUFFIX}"),
        primary_identifier.ident.span(),
    );

    let identifier_comment = &format!(
        "This is a simple wrapper around `Identifier` for the `{target_name}{TABLE_SUFFIX}`. \
        Used to ensure that identifiers for the `{target_name}{TABLE_SUFFIX}` are only used in valid locations."
    );
    quote! {
        #[doc = #identifier_comment]
        pub struct #new_identifier_ident(Identifier);

        impl ToIdentifier for #new_identifier_ident {
            fn to_identifier(&self) -> Identifier {
                self.0
            }
        }
    }
}

fn generate_identifier_generator_definition(target_name: &str, span: Span) -> TokenStream {
    let identifier_ident = Ident::new(&format!("{target_name}{IDENTIFIER_SUFFIX}"), span);
    let identifier_generator_struct_ident =
        Ident::new(&format!("{identifier_ident}{GENERATOR_SUFFIX}"), span);
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

#[cfg(test)]
mod test {

    use crate::msi_tables;
    use pretty_assertions::assert_eq;
    use quote::ToTokens;
    use quote::quote;

    #[test]
    fn test_msi_table() {
        let input = quote! {
            #[MsiTable]
            #[msitable(name = "Directory")]
            struct DirectoryDao {
                #[msitable(primary_key, identifier(generated))]
                directory: DirectoryIdentifier,
                parent_directory: Option<DirectoryIdentifier>,
                #[msi_table(localizable)]
                default_dir: DefaultDir,
            }
        };

        // Call the macro's internal function
        let output = msi_tables::gen_tables_impl(input);

        let expected_output = quote! {
            use whimsi_lib::types::column::identifier::Identifier;
            use whimsi_lib::types::column::identifier::ToIdentifier;
            use whimsi_lib::types::helpers::id_generator::IdentifierGenerator;
            use whimsi_msi::types::helpers::to_msi_value::ToMsiValue;

            #[doc = "This is a simple wrapper around `Identifier` for the `DirectoryTable`. Used to ensure that identifiers for the `DirectoryTable` are only used in valid locations."]
            pub struct DirectoryIdentifier(Identifier);
            impl ToIdentifier for DirectoryIdentifier {
                fn to_identifier(&self) -> Identifier {
                    self.0
                }
            }

            #[derive(Debug, Clone, Default, PartialEq)]
            pub(crate) struct DirectoryIdentifierGenerator {
                count: usize,
                // A reference to a vec of all used Identifiers that should not be generated again.
                // These are all identifiers that inhabit a primary_key column.
                used: std::rc::Rc<std::cell::RefCell<Vec<Identifier>>>,
            }

            impl IdentifierGenerator for DirectoryIdentifierGenerator {
                type IdentifierType = DirectoryIdentifier;

                fn id_prefix(&self) -> &str {
                    "DIRECTORY"
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

            impl From<std::rc::Rc<std::cell::RefCell<Vec<Identifier>>>> for DirectoryIdentifierGenerator {
                fn from(value: std::rc::Rc<std::cell::RefCell<Vec<Identifier>>>) -> Self {
                    let count = value.borrow().len();
                    Self {
                        used: value,
                        count: 0,
                    }
                }
            }

            struct DirectoryDao {
                directory: DirectoryIdentifier,
                parent_directory: Option<DirectoryIdentifier>,
                default_dir: DefaultDir,
            }

            impl PrimaryIdentifier for DirectoryDao {
                fn primary_identifier(&self) -> Identifier {
                    directory.to_identifier()
                }
            }

            impl MsiDao for DirectoryDao {

                fn conflicts_with(&self, other: &Self) -> bool {
                    self.directory == other.directory
                }

                fn to_row(&self) -> Vec<whimsi_msi::Value> {
                    vec![
                        default_dir.to_msi_value(),
                        directory.to_msi_value(),
                        parent_directory.to_msi_value(),
                    ]
                }
            }

            struct DirectoryTable {
                generator: DirectoryIdentifierGenerator,
                entries: Vec<DirectoryDao>,
            }

            impl MsiTable for DirectoryTable {
                fn primary_key_indices(&self) -> Vec<usize> {
                    vec![1]
                }

                fn primary_keys(&self) -> Vec<ColumnTypes> {
                    vec![directory.into()]
                }

                fn columns(&self) -> Vec<whimsi_msi::Column> {
                    vec![
                        whimsi_msi::Column::build("Directory").primary_key().id_string(DEFAULT_IDENTIFIER_MAX_LEN),
                        whimsi_msi::Column::build("Directory_Parent").nullable().id_string(DEFAULT_IDENTIFIER_MAX_LEN),
                        whimsi_msi::Column::build("DefaultDir").localizable().category(whimsi_msi::Category::DefaultDir).string(255),
                    ]
                }
            }

        };

        // Compare the generated output with the expected output (e.g., using syn and comparing ASTs)
        let parsed_output =
            syn::parse2::<syn::File>(output).expect("Failed to parse output of test data");
        let parsed_expected =
            syn::parse2::<syn::File>(expected_output).expect("Failed to parse reference test data");

        assert_eq!(
            parsed_output.to_token_stream().to_string(),
            parsed_expected.to_token_stream().to_string()
        );
    }
}
