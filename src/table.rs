use proc_macro2::TokenStream;
use quote::quote;
use std::str::FromStr;

use crate::{helper::*, msi_tables::FieldInformation};

pub fn generate_table_tokens(target_name: &str, fields: &[FieldInformation]) -> TokenStream {
    let table_definition_tokens = generate_table_definition(target_name);
    let msi_table_impl_tokens = generate_msi_table_impl(target_name, fields);
    quote! {
        #table_definition_tokens
        #msi_table_impl_tokens
    }
}

fn generate_table_definition(target_name: &str) -> TokenStream {
    let table_ident = table_from_name(target_name);
    let dao_type = dao_from_name(target_name);

    quote! {
        #[derive(Clone, Debug, PartialEq)]
        pub struct #table_ident {
            entries: Vec<#dao_type>,
        }
    }
}

fn generate_msi_table_impl(target_name: &str, fields: &[FieldInformation]) -> TokenStream {
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

    let columns = fields.iter().fold(quote! {}, |acc, field| {
        let field_ident = &field.ident.clone().expect("Field doesn't have an identifier");

        let column_name = if let Some(column_name) = &field.column_name {column_name} else { &snake_case_to_pascal_case(&field_ident.to_string())};
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

    let table_name = table_from_name(target_name);
    let dao_name = dao_from_name(target_name);

    quote! {
        impl MsiTableKind for #table_name {
            type TableValue = #dao_name;

            fn name(&self) -> &'static str {
                #target_name
            }

            fn entries(&self) -> &Vec<#dao_name> {
                &self.entries
            }

            fn entries_mut(&mut self) -> &mut Vec<#dao_name> {
                &mut self.entries
            }

            fn primary_key_indices(&self) -> Vec<usize> {
                vec![#primary_key_indices]
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
            let length = field.clone().length.unwrap_or_else(|| {
                panic!(
                    "Field {:?} with category {} must define a length",
                    field.ident, category_str
                )
            });
            quote! {.string(#length)}
        }
    }
}
