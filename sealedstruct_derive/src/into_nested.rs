use quote::quote;
use syn::{parse_macro_input, parse_quote, DeriveInput, WhereClause};

use crate::nested::add_trait_bounds;

pub fn derive_into_nested(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    // Parse the input tokens into a syntax tree.
    let input = parse_macro_input!(input as DeriveInput);

    // Used in the quasi-quotation below as `#name`.
    let struct_name = input.ident;
    let sealable_generics =
        add_trait_bounds(input.generics, &[parse_quote!(sealedstruct::Sealable)]);
    let (impl_generics, ty_generics, where_clause) = sealable_generics.split_for_impl();
    let mut where_clause = where_clause.cloned().unwrap_or(WhereClause {
        predicates: Default::default(),
        where_token: Default::default(),
    });
    where_clause
        .predicates
        .push(parse_quote! {Self: std::cmp::PartialEq});

    let expanded = quote! {

        impl #impl_generics sealedstruct::Sealable for #struct_name #ty_generics #where_clause {
            type Target = Self;

            fn seal(self) -> sealedstruct::Result<Self> {
                Ok(self)
            }

            fn open(sealed: Self) -> Self {
                sealed
            }

            fn partial_eq(&self, other: &Self) -> bool {
                std::cmp::PartialEq::eq(&self, &other)
            }
        }
    };

    // Hand the output tokens back to the compiler.
    proc_macro::TokenStream::from(expanded)
}
