use std::str::FromStr;

use darling::{FromDeriveInput, FromField};
use debug_print::debug_println;
use itertools::Itertools;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::Ident;
use syn::{self};

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
    // -- Builtins ------------------------------------------------------------
    // Field name
    ident: Option<syn::Ident>,
    // Type of the field
    ty: syn::Type,

    // -- Custom --------------------------------------------------------------
    // The category that the given column will be converted to when placed in the table.
    category: syn::Expr,

    // The maximum length of the string placed in the column. This is specific to each table so I
    // can't abstract it away. If it is not provided a default based on the provided Category is
    // used.
    //
    // NOTE: I considered making this optional and using sane defaults for columns
    // based on the given category but I like the idea of not obscuring what values
    // are being used for a given column.
    length: syn::Expr,

    // What the name of the column is. If it is not provided the identifier of the field is
    // converted to title case and underscores are removed.
    #[darling(default)]
    column_name: Option<String>,

    // Denotes if the given field corresponds to a primary key in the table.
    #[darling(default)]
    primary_key: bool,

    // Denotes if the given field is an identifier.
    #[darling(default, rename = "identifier")]
    identifier_options: Option<IdentifierInformation>,

    // Whether or not the given field is localizable as specified in the MSI documentation.
    #[darling(default)]
    localizable: bool,
}

#[derive(darling::FromMeta, FromField, Clone)]
struct IdentifierInformation {
    // Denotes if the given identifier should have a generator created for it.
    #[darling(default)]
    generated: bool,

    // Denotes if the given identifier is a foreign key into the table and if it is, what table the
    // key is from.
    #[darling(default)]
    foreign_key: Option<String>,
}

pub fn gen_tables_impl(input: TokenStream) -> TokenStream {
    let input = syn::parse2::<syn::DeriveInput>(input).unwrap();
    let new_table = NewTableDef::from_derive_input(&input).expect("Failed to parse derive input");

    // Make sure the target name is capitalized. There are no cases where we want
    // structs to be named in snake_case.
    let target_name = helper::capitalize(&new_table.name);

    // TODO: Determine if I should make this also able to take in an enum and if so, how to parse
    // each table from each enum variant using the derive format.
    let struct_data = new_table
        .data
        .take_struct()
        .expect("Generating an MSI table is only supported from a struct currently");

    // Create the table-specific identifier if one should be made. These are made when a table has
    // a column with a type that implements `ToIdentifier` and the column is not marked as a
    // foreign key.
    let primary_identifier = struct_data
        .clone()
        .fields
        .into_iter()
        .filter(|f| {
            f.primary_key
                && f.identifier_options.is_some()
                && f.identifier_options.clone().unwrap().foreign_key.is_none()
        })
        .at_most_one()
        .unwrap_or_else(|_| {
            panic!(
                "More than one primary identifier found in [{}] defintion. This is not supported.",
                new_table.ident
            )
        });

    let identifier_tokens = if let Some(ref primary_identifier) = primary_identifier {
        generate_identifier_tokens(&target_name, primary_identifier)
    } else {
        Default::default()
    };

    let dao_tokens = generate_dao_tokens(&target_name, &primary_identifier, &struct_data.fields);

    let table_tokens = generate_table_tokens(&target_name, &struct_data.fields);

    // Generate the DAO code.
    let output_tokens = quote! {
        use whimsi_lib::types::column::identifier::Identifier;
        use whimsi_lib::types::column::identifier::ToIdentifier;
        use whimsi_lib::types::helpers::id_generator::IdentifierGenerator;
        use msi::types::helpers::to_msi_value::ToMsiValue;

        #identifier_tokens
        #dao_tokens
        #table_tokens
    };

    debug_println!("Macro output: \n{}", output_tokens.to_string());
    output_tokens
}

fn generate_identifier_tokens(
    target_name: &str,
    primary_identifier: &FieldInformation,
) -> TokenStream {
    let primary_identifier_impl_tokens =
        generate_identifier_definition(target_name, primary_identifier);

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
        #primary_identifier_impl_tokens
        #identifier_generator_definition_tokens
    }
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

    let new_identifier_ident = identifier_from_name(target_name);

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

fn generate_dao_tokens(
    target_name: &str,
    primary_identifier: &Option<FieldInformation>,
    fields: &Vec<FieldInformation>,
) -> TokenStream {
    let dao_struct_ident = dao_from_name(target_name);

    let dao_struct_definition_tokens = generate_dao_struct_definition(&dao_struct_ident, fields);
    let primary_identifier_impl_definition_tokens =
        generate_primary_identifier_impl_definition(primary_identifier, &dao_struct_ident);
    let msi_dao_impl_definition_tokens =
        generate_msi_dao_impl_definition(&dao_struct_ident, fields);

    quote! {
        #dao_struct_definition_tokens
        #primary_identifier_impl_definition_tokens
        #msi_dao_impl_definition_tokens
    }
}

fn generate_dao_struct_definition(
    dao_struct_ident: &Ident,
    fields: &Vec<FieldInformation>,
) -> TokenStream {
    // Pretty sure we could just append `fields` to the token stream for this but I want to
    // explicitly drop visibilities here so all properties are private.
    //
    // TODO: This will _not_ propogate proc-macros placed on the fields. Determine if this is
    // needed.
    let mut field_tokens = TokenStream::new();
    for field in fields {
        let field_ident = field.ident.clone();
        let field_type = field.ty.clone();
        field_tokens = quote! {
            #field_tokens
            #field_ident : #field_type ,
        }
    }
    quote! {
        struct #dao_struct_ident {
            #field_tokens
        }
    }
}

fn generate_primary_identifier_impl_definition(
    primary_identifier: &Option<FieldInformation>,
    dao_struct_ident: &Ident,
) -> TokenStream {
    let dao_primary_identifier = match primary_identifier {
        Some(identifier) => {
            let identifier_ident = identifier.ident.clone();
            quote! {#identifier_ident.to_identifier()}
        }
        None => {
            quote! { None }
        }
    };

    quote! {
        impl PrimaryIdentifier for #dao_struct_ident {
            fn primary_identifier(&self) -> Identifier {
                #dao_primary_identifier
            }
        }
    }
}

fn generate_msi_dao_impl_definition(
    dao_struct_ident: &Ident,
    fields: &Vec<FieldInformation>,
) -> TokenStream {
    let conflicts_definition_tokens = generate_msi_dao_conflicts_definition(fields);
    let to_row_definition_tokens = generate_msi_dao_to_row_definition(fields);

    quote! {
        impl MsiDao for #dao_struct_ident {
            #conflicts_definition_tokens
            #to_row_definition_tokens

        }
    }
}

fn generate_msi_dao_conflicts_definition(fields: &Vec<FieldInformation>) -> TokenStream {
    let mut conflict_expression = TokenStream::new();
    // Get the fields that are marked as primary_key as these are what is used to check for
    // conflicts.
    for field in fields {
        if !field.primary_key {
            continue;
        }

        let and_and = if !conflict_expression.is_empty() {
            quote!(&&)
        } else {
            TokenStream::default()
        };

        let field_ident = &field.ident;
        conflict_expression = quote! {
            #conflict_expression
            #and_and self.#field_ident == other.#field_ident
        }
    }

    quote! {
        fn conflicts_with(&self, other: &Self) -> bool {
            #conflict_expression
        }
    }
}

fn generate_msi_dao_to_row_definition(fields: &Vec<FieldInformation>) -> TokenStream {
    let mut fields_to_msi_value_tokens = TokenStream::new();
    for field in fields {
        let field_ident = &field.ident;
        fields_to_msi_value_tokens = quote! {
            #fields_to_msi_value_tokens
            #field_ident.to_msi_value(),
        }
    }

    quote! {
        fn to_row(&self) -> Vec<msi::Value> {
            vec![
                #fields_to_msi_value_tokens
            ]
        }
    }
}

fn generate_table_tokens(target_name: &str, fields: &[FieldInformation]) -> TokenStream {
    let table_definition_tokens = generate_table_definition(target_name, fields);
    let msi_table_impl_tokens = generate_msi_table_impl(fields);
    quote! {
        #table_definition_tokens
        #msi_table_impl_tokens
    }
}

fn generate_table_definition(target_name: &str, fields: &[FieldInformation]) -> TokenStream {
    let table_ident = table_from_name(target_name);
    let dao_type = dao_from_name(target_name);
    let generator_definition = if fields
        .iter()
        .filter_map(|f| f.identifier_options.clone())
        .any(|i| i.generated)
    {
        let generator_type = identifier_generator_from_name(target_name);
        quote! {
            generator: #generator_type,
        }
    } else {
        Default::default()
    };

    quote! {
        struct #table_ident {
            #generator_definition
            entries: Vec<#dao_type>,
        }
    }
}

fn generate_msi_table_impl(fields: &[FieldInformation]) -> TokenStream {
    let primary_key_indices = fields
        .iter()
        .enumerate()
        .fold(quote! {}, |acc, (index, field)| {
            if field.primary_key {
                quote! { #acc #index, }
            } else {
                acc
            }
        });

    let primary_keys = fields.iter().fold(quote! {}, |acc, field| {
        if field.primary_key {
            let field_ident = &field.ident;

            // Primary key fields must implement `Into<ColumnType>`
            quote! {
                #acc
                #field_ident.into(),
            }
        } else {
            acc
        }
    });

    let columns = fields.iter().fold(quote! {}, |acc, field| {
        let field_ident = &field.ident.clone().expect("Field doesn't have an identifier");

        let column_name = if let Some(column_name) = &field.column_name {column_name} else { &helper::snake_case_to_pascal_case(&field_ident.to_string())};
        let nullable = if let syn::Type::Path(path) = &field.ty &&
                 path.path.segments.last().unwrap().ident == "Option" {
                    quote!{.nullable()}
                }
            else {
                Default::default()
            };

        let primary_key = if field.primary_key {
            quote!{.primary_key()}
        } else {
            Default::default()
        };
        let localizable = if field.localizable {
            quote!{.localizable()}
        } else {
            Default::default()
        };

        // If this causes issues it can probably be removed.
        let foreign_key = if let Some(identifier_options) = &field.identifier_options &&
            let Some(foreign_key) = &identifier_options.foreign_key {
            // TODO: This is almost certainly wrong in some circumstance. It assumes that the
            // foreign_key points to the first column of the referenced table. I really want to add
            // a way to dynamically get the primary_key index for the given table, but I would need
            // to split the parsing into 2 sections for that. I might circle back and implement
            // that at some point but I'm gonna skip it for now.
            quote!{.foreign_key(#foreign_key, 0)}
        } else {
            Default::default()
        };

        // TODO: I dislike having to hard code in the `msi` path here but couldn't find a
        // better solution. Should probably look into it some more.
        let field_category = &field.category;
        let category = quote! { .category( #field_category ) };
        let finish = generate_finish_build_for_field(field);

        quote! {
            #acc

            msi::Column::build(#column_name) #primary_key #nullable #localizable #foreign_key #category #finish,
        }
    });

    quote! {
        impl MsiTable for DirectoryTable {
            fn primary_key_indices(&self) -> Vec<usize> {
                vec![#primary_key_indices]
            }

            fn primary_keys(&self) -> Vec<ColumnType> {
                vec![#primary_keys]
            }

            fn columns(&self) -> Vec<msi::Column> {
                vec![
                    #columns
                ]
            }
        }
    }
}

fn generate_finish_build_for_field(field: &FieldInformation) -> TokenStream {
    let syn::Expr::Path(ref path) = field.category else {
        panic!("Category is not a valid syn::Expr::Path.")
    };
    let category_str = path
        .path
        .segments
        .last()
        .expect("Path contains no segments")
        .ident
        .to_string();
    let category = msi::Category::from_str(&category_str)
        .unwrap_or_else(|_| panic!("Category is invalid: {}", category_str));
    match category {
        msi::Category::Integer => quote! {.int16()},
        msi::Category::DoubleInteger => quote! {.int32()},
        _ => {
            let length = &field.length;
            quote! {.string(#length)}
        }
    }
}

fn dao_from_name(target_name: &str) -> Ident {
    format_ident!("{target_name}{DAO_SUFFIX}")
}

fn table_from_name(target_name: &str) -> Ident {
    format_ident!("{target_name}{TABLE_SUFFIX}")
}

fn identifier_from_name(target_name: &str) -> Ident {
    format_ident!("{target_name}{IDENTIFIER_SUFFIX}")
}

fn identifier_generator_from_name(target_name: &str) -> Ident {
    let identifier = identifier_from_name(target_name);
    format_ident!("{identifier}{GENERATOR_SUFFIX}")
}

#[cfg(test)]
mod test {
    use crate::msi_tables;
    use pretty_assertions::assert_eq;
    use quote::ToTokens;
    use quote::quote;

    #[test]
    fn test_msi_table_with_generated_identifier() {
        let input = quote! {
            #[MsiTable]
            #[msitable(name = "Directory")]
            struct DirectoryDao {
                #[msitable(primary_key, identifier(generated), category = msi::Category::Identifier, length = 72)]
                directory: DirectoryIdentifier,
                #[msitable(identifier(foreign_key = "Directory"), column_name = "Directory_Parent", category = msi::Category::Identifier, length = 72)]
                parent_directory: Option<DirectoryIdentifier>,
                #[msitable(localizable, category = msi::Category::DefaultDir, length = 255)]
                default_dir: DefaultDir,
            }
        };

        // Call the macro's internal function
        let output = msi_tables::gen_tables_impl(input);

        let expected_output = quote! {
            use whimsi_lib::types::column::identifier::Identifier;
            use whimsi_lib::types::column::identifier::ToIdentifier;
            use whimsi_lib::types::helpers::id_generator::IdentifierGenerator;
            use msi::types::helpers::to_msi_value::ToMsiValue;

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

                fn to_row(&self) -> Vec<msi::Value> {
                    vec![
                        directory.to_msi_value(),
                        parent_directory.to_msi_value(),
                        default_dir.to_msi_value(),
                    ]
                }
            }

            struct DirectoryTable {
                generator: DirectoryIdentifierGenerator,
                entries: Vec<DirectoryDao>,
            }

            impl MsiTable for DirectoryTable {
                fn primary_key_indices(&self) -> Vec<usize> {
                    vec![0usize,]
                }

                fn primary_keys(&self) -> Vec<ColumnType> {
                    vec![directory.into(),]
                }

                fn columns(&self) -> Vec<msi::Column> {
                    vec![
                        msi::Column::build("Directory").primary_key().category(msi::Category::Identifier).string(72),
                        msi::Column::build("Directory_Parent").nullable().foreign_key("Directory", 0).category(msi::Category::Identifier).string(72),
                        msi::Column::build("DefaultDir").localizable().category(msi::Category::DefaultDir).string(255),
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

    #[test]
    fn test_msi_table_without_generated_identifier() {
        let input = quote! {
            #[MsiTable]
            #[msitable(name = "FeatureComponent")]
            struct FeatureComponentDao {
                #[msitable(primary_key, identifier(foreign_key = "Feature"), category = msi::Category::Identifier, length = 72)]
                feature_: FeatureIdentifier,
                #[msitable(primary_key, identifier(foreign_key = "Component"), category = msi::Category::Identifier, length = 72)]
                component_: ComponentIdentifier,
            }
        };

        // Call the macro's internal function
        let output = msi_tables::gen_tables_impl(input);

        let expected_output = quote! {
            use whimsi_lib::types::column::identifier::Identifier;
            use whimsi_lib::types::column::identifier::ToIdentifier;
            use whimsi_lib::types::helpers::id_generator::IdentifierGenerator;
            use msi::types::helpers::to_msi_value::ToMsiValue;

            struct FeatureComponentDao {
                feature_: FeatureIdentifier,
                component_: ComponentIdentifier,
            }

            impl PrimaryIdentifier for FeatureComponentDao {
                fn primary_identifier(&self) -> Identifier {
                    None
                }
            }

            impl MsiDao for FeatureComponentDao {

                fn conflicts_with(&self, other: &Self) -> bool {
                    self.feature_ == other.feature_ && self.component_ == other.component_
                }

                fn to_row(&self) -> Vec<msi::Value> {
                    vec![
                        feature_.to_msi_value(),
                        component_.to_msi_value(),
                    ]
                }
            }

            struct FeatureComponentTable {
                entries: Vec<FeatureComponentDao>,
            }

            impl MsiTable for DirectoryTable {
                fn primary_key_indices(&self) -> Vec<usize> {
                    vec![0usize,1usize,]
                }

                fn primary_keys(&self) -> Vec<ColumnType> {
                    vec![feature_.into(),component_.into(),]
                }

                fn columns(&self) -> Vec<msi::Column> {
                    vec![
                        msi::Column::build("Feature_").primary_key().foreign_key("Feature", 0).category(msi::Category::Identifier).string(72),
                        msi::Column::build("Component_").primary_key().foreign_key("Component", 0).category(msi::Category::Identifier).string(72),
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
