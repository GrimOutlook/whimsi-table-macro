use crate::helper::*;
use crate::msi_tables::FieldInformation;
use itertools::Itertools;
use proc_macro2::TokenStream;
use quote::quote;
use syn::Ident;

pub fn generate_dao_tokens(
    target_name: &str,
    primary_identifier: &Option<&FieldInformation>,
    fields: &Vec<FieldInformation>,
) -> TokenStream {
    let dao_struct_ident = dao_from_name(target_name);

    let dao_struct_tokens = generate_dao_struct_definition(&dao_struct_ident, fields);
    let dao_impl_tokens = generate_new_for_dao(target_name, fields);
    let primary_identifier_impl_tokens =
        generate_primary_identifier_impl_definition(primary_identifier, &dao_struct_ident);
    let msi_dao_impl_tokens = generate_msi_dao_impl_definition(&dao_struct_ident, fields);

    quote! {
        #dao_struct_tokens
        #dao_impl_tokens
        #primary_identifier_impl_tokens
        #msi_dao_impl_tokens
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

        #[derive(Clone, Debug, PartialEq, getset::Getters)]
        #[getset(get = "pub")]
        pub struct #dao_struct_ident {
            #field_tokens
        }
    }
}

fn generate_primary_identifier_impl_definition(
    primary_identifier: &Option<&FieldInformation>,
    dao_struct_ident: &Ident,
) -> TokenStream {
    let dao_primary_identifier = match primary_identifier {
        Some(identifier) => {
            let identifier_ident = identifier.ident.clone();
            quote! { Some( self.#identifier_ident.to_identifier() ) }
        }
        None => {
            quote! { None }
        }
    };

    quote! {
        impl PrimaryIdentifier for #dao_struct_ident {
            fn primary_identifier(&self) -> Option<Identifier> {
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
            msi::ToValue::to_value(&self.#field_ident),
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

fn generate_new_for_dao(target_name: &str, fields: &[FieldInformation]) -> TokenStream {
    let field_idents = fields
        .iter()
        .map(|f| f.ident.clone().expect("Field didn't have an identifier"))
        .collect_vec();
    let field_types = fields.iter().map(|f| f.ty.clone()).collect_vec();
    let dao_name = dao_from_name(target_name);
    quote! {
        impl #dao_name {
            pub fn new( #(#field_idents: impl Into<#field_types>),* ) -> #dao_name {
                #dao_name { #(#field_idents: #field_idents.into()),* }
            }
        }
    }
}
