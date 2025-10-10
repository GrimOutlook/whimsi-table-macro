use darling::{FromDeriveInput, FromField, FromVariant};
use itertools::Itertools;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{self};

use crate::{
    dao::generate_dao_tokens, helper::*, identifier::generate_identifier_tokens,
    table::generate_table_tokens,
};

#[derive(FromDeriveInput, Clone)]
#[darling(attributes(msi_table))]
pub(crate) struct DeriveInformation {
    pub ident: syn::Ident,
    pub data: darling::ast::Data<VariantInformation, FieldInformation>,

    // WARN: ONLY USED IF DERIVED ITEM IS A `struct`!
    //
    // If this is a struct, the base name of the table to create. EX: "Directory" will produces
    // struct names such as "DirectoryDao" and "DirectoryTable".
    pub name: Option<String>,
}

#[derive(FromVariant, Clone)]
pub(crate) struct VariantInformation {
    pub ident: syn::Ident,
    pub fields: darling::ast::Fields<FieldInformation>,
}

#[derive(FromField, Clone)]
#[darling(attributes(msi_column))]
pub(crate) struct FieldInformation {
    // -- Builtins ------------------------------------------------------------
    // Field name
    pub ident: Option<syn::Ident>,
    // Type of the field
    pub ty: syn::Type,

    // -- Custom --------------------------------------------------------------
    // The category that the given column will be converted to when placed in the table.
    pub category: syn::Expr,

    // The maximum length of the string placed in the column. This is specific to each table so I
    // can't abstract it away. If it is not provided a default based on the provided Category is
    // used.
    //
    // NOTE: I considered making this optional and using sane defaults for columns
    // based on the given category but I like the idea of not obscuring what values
    // are being used for a given column. This is only optional for categories of Integer and
    // DoubleInteger.
    pub length: Option<syn::Expr>,

    // What the name of the column is. If it is not provided the identifier of the field is
    // converted to title case and underscores are removed.
    #[darling(default)]
    pub column_name: Option<String>,

    // Denotes if the given field corresponds to a primary key in the table.
    #[darling(default)]
    pub primary_key: bool,

    // Denotes if the given field is an identifier.
    #[darling(default, rename = "identifier")]
    pub identifier_options: Option<IdentifierInformation>,

    // Whether or not the given field is localizable as specified in the MSI documentation.
    #[darling(default)]
    pub localizable: bool,
}

#[derive(darling::FromMeta, FromField, Clone)]
pub(crate) struct IdentifierInformation {
    // Denotes if the given identifier should have a generator created for it.
    #[darling(default)]
    pub generated: bool,

    // Denotes if the given identifier is a foreign key into the table and if it is, what table the
    // key is from.
    #[darling(default)]
    pub foreign_key: Option<String>,
}

pub fn gen_tables_impl(input: TokenStream) -> TokenStream {
    let input = syn::parse2::<syn::DeriveInput>(input).unwrap();
    let derive_input =
        DeriveInformation::from_derive_input(&input).expect("Failed to parse derive input");

    let output_tokens = match derive_input.data {
        darling::ast::Data::Enum(items) => {
            gen_tables_for_enum(&derive_input.ident.to_string(), items)
        }
        darling::ast::Data::Struct(fields) => {
            let name = capitalize(&derive_input.name.unwrap_or(derive_input.ident.to_string()));
            gen_tables_for_fields(&name, fields.fields)
        }
    };

    quote! {
        use whimsi_lib::types::column::identifier::Identifier;
        use whimsi_lib::types::column::identifier::ToIdentifier;

        #output_tokens
    }
}

fn gen_tables_for_enum(name: &str, items: Vec<VariantInformation>) -> TokenStream {
    let (struct_variants, dao_variants) = items
        .iter()
        .map(|v| {
            let variant = v.ident.clone();
            let table_name = table_from_name(&variant.to_string());
            let dao_name = dao_from_name(&variant.to_string());
            (
                quote! { #variant ( #table_name ) , },
                quote! { #variant ( #dao_name ) , },
            )
        })
        .collect::<(Vec<TokenStream>, Vec<TokenStream>)>();

    // Generate the enum containing all of the variant structs
    let table_enum_name = format_ident!("{name}");
    let dao_enum_name = dao_from_name(name);
    let tokens = quote! {
        #[derive(Clone, PartialEq, strum::EnumDiscriminants, derive_more::Into, derive_more::From, derive_more::TryFrom, derive_more::TryInto, strum::Display)]
        #[strum_discriminants(name(MsiTable))]
        pub enum #table_enum_name {
            #(#struct_variants)*
        }

        #[derive(Clone, PartialEq)]
        pub enum #dao_enum_name {
            #(#dao_variants)*
        }
    };
    items.iter().fold(tokens, |acc, variant| {
        let table_def_tokens =
            gen_tables_for_fields(&variant.ident.to_string(), variant.fields.fields.clone());
        quote! {
            #acc
            #table_def_tokens
        }
    })
}

fn gen_tables_for_fields(base_name: &str, fields: Vec<FieldInformation>) -> TokenStream {
    let target_name = capitalize(base_name);

    // Create the table-specific identifier if one should be made. These are made when a table has
    // a column with a type that implements `ToIdentifier` and the column is not marked as a
    // foreign key.
    let primary_identifier = fields
        .iter()
        .filter(|f| {
            f.primary_key
                && f.identifier_options.is_some()
                && f.identifier_options.clone().unwrap().foreign_key.is_none()
        })
        .at_most_one()
        .unwrap_or_else(|_| {
            panic!(
                "More than one field marked as primary identifier found in defintion. This is not supported."
            )
        });

    let identifier_tokens = if let Some(primary_identifier) = primary_identifier {
        generate_identifier_tokens(&target_name, primary_identifier)
    } else {
        Default::default()
    };

    let dao_tokens = generate_dao_tokens(&target_name, &primary_identifier, &fields);

    let table_tokens = generate_table_tokens(&target_name, &fields);

    // Generate the DAO code.
    let output_tokens = quote! {
        #identifier_tokens
        #dao_tokens
        #table_tokens
    };

    output_tokens
}

#[cfg(test)]
mod tests;
