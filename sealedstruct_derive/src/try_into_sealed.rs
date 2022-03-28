use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::spanned::Spanned;
use syn::{parse_macro_input, Data, DeriveInput, Fields, Ident};

pub fn derive_try_into_sealed(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    // Parse the input tokens into a syntax tree.
    let input = parse_macro_input!(input as DeriveInput);

    // Used in the quasi-quotation below as `#name`.
    let raw_struct_name = input.ident;
    let raw_struct_name_str = raw_struct_name.to_string();
    if !raw_struct_name_str.ends_with("Raw") {
        panic!("Struct name must end with 'Raw'");
    }

    let struct_name_str = &raw_struct_name_str[..(raw_struct_name_str.len() - 3)];
    let struct_name = syn::Ident::new(struct_name_str, raw_struct_name.span());
    let sealed_name = syn::Ident::new(&format!("{struct_name}Sealed"), raw_struct_name.span());
    let result_name = syn::Ident::new(&format!("{struct_name}Result"), raw_struct_name.span());

    // Generate an expression to sum up the heap size of each field.
    let result = create_fields(&input.data, &result_name);

    let expanded = quote! {
        impl sealedstruct::TryIntoSealed for #raw_struct_name {
            type Target = #sealed_name;

            fn try_into_sealed(self) -> sealedstruct::Result<Self::Target> {
                #result.into()
            }
        }
    };

    // Hand the output tokens back to the compiler.
    proc_macro::TokenStream::from(expanded)
}

// Generate an expression to sum up the heap size of each field.
fn create_fields(data: &Data, result_name: &Ident) -> TokenStream {
    match *data {
        Data::Struct(ref data) => {
            match data.fields {
                Fields::Named(ref fields) => {
                    // Expands to an expression like
                    //
                    //     0 + self.x.heap_size() + self.y.heap_size() + self.z.heap_size()
                    //
                    // but using fully qualified function call syntax.
                    //
                    // We take some care to use the span of each `syn::Field` as
                    // the span of the corresponding `heap_size_of_children`
                    // call. This way if one of the field types does not
                    // implement `HeapSize` then the compiler's error message
                    // underlines which field it is. An example is shown in the
                    // readme of the parent directory.

                    let recurse = fields.named.iter().map(|f| {
                        let name = &f.ident;
                        let name_str = name.as_ref().expect("Has ident").to_string();
                        quote_spanned! {f.span()=>
                            #name: sealedstruct::prelude::ValidationResultExtensions::prepend_path(
                                sealedstruct::TryIntoSealed::try_into_sealed(self.#name),
                                #name_str
                            ),
                        }
                    });
                    quote! {
                        #result_name {
                            #(#recurse)*
                        }
                    }
                }
                Fields::Unnamed(ref _fields) => {
                    unimplemented!("Tuple-Structs are not supported yet");
                }
                Fields::Unit => {
                    // Unit structs cannot own more than 0 bytes of heap memory.
                    quote!()
                }
            }
        }
        Data::Enum(ref e) => {
            let field_mappings = e.variants.iter().map(|v| {
                let ident = &v.ident;
                match &v.fields {
                    &Fields::Unit => quote! {
                        Self::#ident => #result_name::#ident,
                    },
                    _ => unimplemented!("Just unit fields are supported"),
                }
            });
            quote! {
                match self {
                    #(#field_mappings)*
                }
            }
        }
        Data::Union(_) => unimplemented!(),
    }
}
