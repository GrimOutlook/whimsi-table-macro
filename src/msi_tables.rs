use darling::{FromDeriveInput, FromField};
use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{DeriveInput, punctuated::Punctuated};

#[derive(FromDeriveInput, Clone)]
#[darling(attributes(msitable))]
struct NewTableDef {
    data: darling::ast::Data<(), FieldInformation>,

    name: Option<String>,
    primary_key: Option<bool>,
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
    #[darling(default)]
    identifier: Option<IdentifierInformation>,
}

#[derive(darling::FromMeta, FromField, Clone)]
struct IdentifierInformation {
    // Denotes that the data must only exist once in a package_unique column in the MSI.
    //
    // For example, if the identifier is in the Directory table's `Directory` column, it cannot
    // exist in the `File` table's `File` column as well. It can only exist in one of these.
    //
    // TODO: Determine if this can be jetisoned. It seems like this is just an alias for
    // `primary_key` && `identifier`. If I can find a counterexample this can stay but
    // it should be removed otherwise.
    #[darling(default)]
    package_unique: bool,

    // Denotes if the given identifier should have a generator created for it.
    #[darling(default)]
    generated: bool,
}

pub fn gen_tables_impl(input: TokenStream) -> TokenStream {
    let input = syn::parse2::<syn::DeriveInput>(input).unwrap();
    let new_table = NewTableDef::from_derive_input(&input).expect("Failed to parse derive input");

    println!("Name: {:?}", new_table.name);
    println!("Primary Key: {:?}", new_table.primary_key);
    let struct_data = new_table.data.take_struct().unwrap();
    println!(
        "Fields: {:?}",
        struct_data
            .fields
            .iter()
            .map(|f| f.ident.to_token_stream().to_string())
            .collect::<Vec<_>>()
            .join(", ")
    );
    for field in struct_data.fields {
        println!("Ident: {:?}- Primary: {:?}", field.ident, field.primary_key);
        if let Some(id) = field.identifier {
            println!("Generated: {}", id.generated);
        }
    }
    quote!()
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
                #[msi_table(localizable)]
                default_dir: DefaultDir,
                #[msitable(primary_key, identifier(generated, package_unique))]
                directory: DirectoryIdentifier,
                parent_directory: Option<DirectoryIdentifier>,
            }
        };

        // Call the macro's internal function
        let output = msi_tables::gen_tables_impl(input);

        let expected_output = quote! {
            use whimsi_lib::types::column::identifier::Identifier;
            use whimsi_lib::types::column::identifier::ToIdentifier;
            use whimsi_lib::types::helpers::id_generator::IdentifierGenerator;
            use whimsi_msi::types::helpers::to_msi_value::ToMsiValue;

            struct DirectoryIdentifier(Identifier);
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
                default_dir: DefaultDir,
                directory: DirectoryIdentifier,
                parent_directory: Option<DirectoryIdentifier>
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
                generator:
                entries: Vec<DirectoryDao>
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
        let parsed_output = syn::parse2::<syn::File>(output).unwrap();
        let parsed_expected = syn::parse2::<syn::File>(expected_output).unwrap();

        assert_eq!(
            parsed_output.to_token_stream().to_string(),
            parsed_expected.to_token_stream().to_string()
        );
    }
}
