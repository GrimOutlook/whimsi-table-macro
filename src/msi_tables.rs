use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::punctuated::Punctuated;

pub fn gen_tables_impl(input: TokenStream) -> TokenStream {
    let input = syn::parse2::<syn::ItemStruct>(input).unwrap();
    // Categorize the attributes to simplify later use
    let mut table_name: Option<String> = None;
    for attr in input.attrs {
        if attr.path().is_ident("msitable") {
            let attr_args = attr
                .parse_args_with(Punctuated::<syn::Meta, syn::Token![,]>::parse_terminated)
                .expect("Failed to parse attribute arguments");
            for attr_arg in attr_args {
                if attr_arg.path().is_ident("name") {
                    table_name = Some(
                        attr_arg
                            .require_name_value()
                            .unwrap()
                            .value
                            .to_token_stream()
                            .to_string(),
                    )
                }
            }
        }
    }

    println!("Name: {:?}", table_name);

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
                default_dir: DefaultDir,
                #[msitable(primary_key, generated, table_unique, package_unique)]
                directory: DirectoryIdentifier,
                parent_directory: Option<DirectoryIdentifier>,
            }
        };

        // Call the macro's internal function (assuming it's public for testing)
        let output = msi_tables::gen_tables_impl(input);

        let expected_output = quote! {
            struct DirectoryDao {
                default_dir: DefaultDir,
                directory: DirectoryIdentifier,
                parent_directory: Option<DirectoryIdentifier>
            }

            impl MsiTable for DirectoryIdentifier {
                fn primary_key_indices(&self) -> Vec<usize> {
                    vec![1]
                }

                fn primary_keys(&self) -> Vec<ColumnTypes> {
                    vec![directory.into()]
                }

                fn conflicts_with(&self, other: &Self) -> bool {
                    self.directory == other.directory
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
