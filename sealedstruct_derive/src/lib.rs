mod seal;
mod try_into_sealed;

/// Generetes several other structs based on {Structname}Raw
///  - {Structname}Raw: Can be deserialized or manually constructed. All fields can be pub.
///  - {Structname}Sealed: Contains a {Structname} on which it implements deref.
///    It shouldn't be possible to generate a Sealed-instance without raw::try_into_sealed()
///    If Raw is clone/copy, Sealed should have the same behavior
///  - {Structname}: All fields are public. It's fields should only be accessed by {Structname}Sealed
///  - {Structname}Result: Helper which can be used inside TryIntoSealed to turn {StructName}Raw
///    into Result<{StructName}Sealed, ValidationErrors>. It is private to the file in which
///    it is generated on purpose.
#[proc_macro_derive(Seal)]
pub fn derive_seal(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    seal::derive_seal(input)
}

/// Generates a TryIntoSealed implementation by forwarding all errors from subfields.
/// All subfields therefore have to implement TryIntoSealed
#[proc_macro_derive(TryIntoSealed)]
pub fn derive_try_into_sealed(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    try_into_sealed::derive_try_into_sealed(input)
}
 