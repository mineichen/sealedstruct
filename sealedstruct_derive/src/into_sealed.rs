use quote::quote;
use syn::{parse_macro_input, DeriveInput};

pub fn derive_into_sealed(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    // Parse the input tokens into a syntax tree.
    let input = parse_macro_input!(input as DeriveInput);

    // Used in the quasi-quotation below as `#name`.
    let struct_name = input.ident;

    let expanded = quote! {
        impl sealedstruct::TryIntoSealed for #struct_name {
            type Target = Self;

            fn try_into_sealed(self) -> sealedstruct::Result<Self::Target> {
                Ok(self)
            }
        }
    };

    // Hand the output tokens back to the compiler.
    proc_macro::TokenStream::from(expanded)
}
