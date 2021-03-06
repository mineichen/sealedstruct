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
    let inner_name = syn::Ident::new(&format!("{struct_name}"), raw_struct_name.span());
    let result_name = syn::Ident::new(&format!("{struct_name}Result"), raw_struct_name.span());

    // Generate an expression to sum up the heap size of each field.
    let result = create_fields(&input.data, &result_name);

    let expanded = quote! {
        impl sealedstruct::TryIntoSealed for #raw_struct_name {
            type Target = #inner_name;

            fn try_into_sealed(self) -> sealedstruct::Result<Self::Target> {
                #result
            }
        }
    };

    // Hand the output tokens back to the compiler.
    proc_macro::TokenStream::from(expanded)
}

fn create_fields(data: &Data, result_name: &Ident) -> TokenStream {
    match *data {
        Data::Struct(ref data) => {
            match data.fields {
                Fields::Named(ref fields) => {
                    let recurse = fields.named.iter().map(|f| {
                        let name = &f.ident;
                        quote_spanned! {f.span()=>
                            #name: sealedstruct::Sealable::seal(self.#name),
                        }
                    });
                    quote! {
                        #result_name {
                            #(#recurse)*
                        }.into()
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
                }.into()
            }
        }
        Data::Union(_) => unimplemented!(),
    }
}
