use quote::quote;
use syn::{parse_macro_input, DeriveInput};

pub fn derive_into_sealed(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    // Parse the input tokens into a syntax tree.
    let input = parse_macro_input!(input as DeriveInput);

    // Used in the quasi-quotation below as `#name`.
    let struct_name = input.ident;

    let expanded = quote! {
        impl sealedstruct::Sealable for #struct_name {
            type Target = Self;

            fn seal(self) -> sealedstruct::Result<Self> {
                Ok(self)
            }

            fn open(sealed: Self) -> Self {
                sealed
            }

            fn partial_eq(&self, other: &Self) -> bool {
                self.eq(other)
            }
        }
    };

    // Hand the output tokens back to the compiler.
    proc_macro::TokenStream::from(expanded)
}
