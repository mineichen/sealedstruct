use quote::quote;
use syn::{parse_macro_input, DeriveInput};

pub fn derive_validator(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    // Parse the input tokens into a syntax tree.
    let input = parse_macro_input!(input as DeriveInput);

    // Used in the quasi-quotation below as `#name`.
    let struct_name = input.ident;

    let expanded = quote! {
        impl sealedstruct::Validator for #struct_name {
            fn check(&self) ->  sealedstruct::Result<()> {
                Ok(())
            }
        }
    };

    // Hand the output tokens back to the compiler.
    proc_macro::TokenStream::from(expanded)
}
