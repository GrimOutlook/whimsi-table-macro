use crate::constants::*;
use fancy_regex::Regex;
use itertools::Itertools;
use quote::format_ident;
use std::sync::LazyLock;
use syn::Ident;

/// Capitalizes the first character in s.
/// Shamelessly stolen from [here](https://nick.groenen.me/notes/capitalize-a-string-in-rust/)
/// which attributes it to [here](https://stackoverflow.com/a/38406885)
///
/// Why tf is this not in std?
pub fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}

// NOTE: I can already see a bug here but I don't care at the moment. If the word starts with an
// `_` the `_` will be correctly preserved but since the first character in the string is `_` no
// capitalization will occur, when likely you would want the first letter after it to be
// capitalized. Maybe I'll fix it later :p
pub fn snake_case_to_pascal_case(s: &str) -> String {
    static RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?<!^)_(?!$)").unwrap());
    RE.split(s)
        .map_ok(capitalize)
        .collect::<Result<String, fancy_regex::Error>>()
        .unwrap_or_else(|_| panic!("Failed to capitalize string: [{}]", s))
}

pub fn dao_from_name(target_name: &str) -> Ident {
    format_ident!("{target_name}{DAO_SUFFIX}")
}

pub fn table_from_name(target_name: &str) -> Ident {
    format_ident!("{target_name}{TABLE_SUFFIX}")
}

pub fn identifier_from_name(target_name: &str) -> Ident {
    format_ident!("{target_name}{IDENTIFIER_SUFFIX}")
}

pub fn identifier_generator_from_name(target_name: &str) -> Ident {
    let identifier = identifier_from_name(target_name);
    format_ident!("{identifier}{GENERATOR_SUFFIX}")
}
