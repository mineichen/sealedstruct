mod into_nested;
mod nested;
mod seal;
mod try_into_nested;

/// Generetes several other structs based on {Structname}Raw
///  - {Structname}Raw: Can be deserialized or manually constructed. All fields can be pub.
///  - {Structname}Sealed: Contains a {Structname} on which it implements deref.
///    It shouldn't be possible to generate a Sealed-instance without raw::try_into_nested()
///    If Raw is clone/copy, Sealed should have the same behavior
///  - {Structname}: All fields are public. It's fields should only be accessed by {Structname}Sealed
///  - {Structname}Result: Helper which can be used inside TryIntoNested to turn {StructName}Raw
///    into Result<{StructName}Sealed, ValidationErrors>. It is private to the file in which
///    it is generated on purpose.
#[proc_macro_derive(Nested, attributes(sealedDerive))]
pub fn derive_nested(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    nested::derive_seal(input)
}
#[proc_macro_derive(Seal)]
pub fn derive_seal(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    seal::derive_seal(input)
}

/// Generates a TryIntoNested implementation by forwarding all errors from subfields.
/// All subfields therefore have to implement TryIntoNested
#[proc_macro_derive(TryIntoNested)]
pub fn derive_try_into_nested(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    try_into_nested::derive_try_into_nested(input)
}

/// Implements TryIntoNested for a types without invalid invariant
#[proc_macro_derive(IntoNested)]
pub fn derive_into_nested(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    into_nested::derive_into_nested(input)
}
