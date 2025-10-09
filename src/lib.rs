// NOTE: Objectives that this macro must handle:
// - Create a valid Table object from the given inputs.
//     - Determine what the stored datatypes are.
//     - Determine what datatype the stored data must be converted into for insertion.
//         - Just make From<T> to Value a constraint on stored datatypes and we can just use
//         `.into()`.
//     - Determine which columns are primary keys.
//     - Determine which columns are nullable.
//         - Wrap in `Option`?
//     - Determine which columns are generated on the fly and which need to be accepted in the
//     constructor.
//     - Determine which columns must have unique values for each row in the table.
//     - Determine which columns must be unique accross the MSI.
//     - Allow custom implementations for insertion into certain tables.
use syn::parse_macro_input;

extern crate proc_macro;

mod constants;
mod helper;
mod msi_tables;

#[proc_macro]
pub fn msi_table_list(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input);
    msi_tables::gen_tables_impl(input).into()
}

// The `gen_tables_impl` function handles both enums and structs so we can call it for both single
// table generation and multi table generation.
#[proc_macro]
pub fn msi_table(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    msi_table_list(input)
}
