use proc_macro2::{Ident, TokenStream};
use quote::{quote, quote_spanned};
use syn::spanned::Spanned;
use syn::{parse_macro_input, Data, DeriveInput, Fields, Index};

pub fn derive_seal(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    // Parse the input tokens into a syntax tree.
    let input = parse_macro_input!(input as DeriveInput);
    if let syn::Visibility::Inherited = input.vis {
        panic!("Raw-Struct mustn't be private. Deriving 'Seal' only makes sense if generated Sealed* is in submodule");
    }

    // Used in the quasi-quotation below as `#name`.
    let raw_name = input.ident;
    let raw_name_str = raw_name.to_string();
    if !raw_name_str.ends_with("Raw") {
        panic!("Struct name must end with 'Raw'");
    }

    let (impl_generics, ty_generics, _) = input.generics.split_for_impl();
    let struct_name_str = &raw_name_str[..(raw_name_str.len() - 3)];
    let facade_name = syn::Ident::new(struct_name_str, raw_name.span());
    let wrapper_name = syn::Ident::new(&format!("{struct_name_str}Wrapper"), raw_name.span());
    let result_name = syn::Ident::new(&format!("{struct_name_str}Result"), raw_name.span());
    let input_vis = input.vis;

    // Generate an expression to sum up the heap size of each field.
    let result = create_result(&input.data, quote! { #result_name });
    let result_into_wrapper =
        create_result_into_wrapper_body(&input.data, &wrapper_name, &raw_name, &result_name);

    #[cfg(feature = "serde")]
    let serde_wrapper = {
        quote! {
            impl<T: serde::Serialize> serde::Serialize for #wrapper_name<T>  {
                fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
                where
                    S: serde::Serializer,
                {
                    self.0.serialize(serializer)
                }
            }
            impl<'de, T: serde::Deserialize<'de> + sealedstruct::Validator> serde::Deserialize<'de> for #wrapper_name<T> {
                fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
                where
                    D: serde::Deserializer<'de>,
                {
                    T::deserialize(deserializer).and_then(|e| {
                        e.check().map_err(<D::Error as serde::de::Error>::custom)?;
                        Ok(#wrapper_name(e))
                    })
                }
            }
        }
    };
    #[cfg(not(feature = "serde"))]
    let serde_wrapper = quote! {};

    let expanded = quote! {
        #result

        #serde_wrapper

        #input_vis type #facade_name #ty_generics = #wrapper_name<#raw_name #ty_generics>;
        impl #impl_generics TryFrom<#raw_name  #ty_generics> for #facade_name  #ty_generics {
            type Error = sealedstruct::ValidationErrors;

            fn try_from(value: #raw_name  #ty_generics) -> Result<Self, Self::Error> {
                sealedstruct::Validator::check(&value)?;
                Ok(#wrapper_name(value))
            }
        }

        impl #impl_generics  #raw_name #ty_generics {
            pub fn seal(self) -> sealedstruct::Result<#facade_name #ty_generics> {
                self.try_into()
            }
        }

        impl #impl_generics #wrapper_name<#raw_name #ty_generics> {
            fn new_unchecked(raw: #raw_name #ty_generics) -> Self {
                #[cfg(debug_assertions)]
                if let Err(e) = sealedstruct::Validator::check(&raw) {
                    panic!("Bug: new_unchecked is expected to receive valid values: {e}");
                }
                Self(raw)
            }

            pub fn into_inner(self) -> #raw_name #ty_generics {
                self.0
            }
        }

        impl<T: std::fmt::Display> std::fmt::Display for #wrapper_name<T> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                std::fmt::Display::fmt(&self.0, f)
            }
        }

        #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Default, sealedstruct::IntoNested, Ord, PartialOrd)]
        pub struct #wrapper_name<T>(T);

        impl<T> std::ops::Deref for #wrapper_name<T> {
            type Target = T;

            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl From<#result_name> for sealedstruct::Result<()> {
            fn from(input: #result_name) -> Self {
                #result_into_wrapper
            }
        }
    };

    // Hand the output tokens back to the compiler.
    proc_macro::TokenStream::from(expanded)
}

fn create_result_into_wrapper_body(
    data: &Data,
    wrapper_name: &Ident,
    raw_name: &Ident,
    result_name: &Ident,
) -> TokenStream {
    match *data {
        Data::Struct(ref data) => {
            match data.fields {
                Fields::Named(ref fields) => {
                    let mut ident_iter = fields.named.iter().flat_map(|f| f.ident.clone());
                    match ident_iter.next() {
                        Some(first) => {
                            let first_string = first.to_string();
                            let assign = ident_iter.fold(
                                quote!{
                                    sealedstruct::prelude::ValidationResultExtensions::prepend_path(input.#first, #first_string)
                                },
                                |assign, next| {
                                    let next_text = next.to_string();

                                    quote! { sealedstruct::prelude::ValidationResultExtensions::combine(#assign,
                                        sealedstruct::prelude::ValidationResultExtensions::prepend_path(input.#next, #next_text))
                                    }
                                },
                            );
                            // Generates e.g.:
                            // input.foo.prependPath("foo").combine(input.bar.prependPath("bar")).combine(input.baz.prepend_path("baz"));
                            quote! {
                                #assign.map(|_| ())
                            }
                        }
                        _ => quote! { Ok(())},
                    }
                }
                Fields::Unnamed(ref fields) => {
                    let mut ident_iter = (0..fields.unnamed.len()).map(|f| {
                        let index = Index::from(f);
                        (quote! { input.#index }, f.to_string())
                    });
                    match ident_iter.next() {
                        Some((first_acc, first_label)) => {
                            let assign = ident_iter.fold(
                                quote!{
                                    sealedstruct::prelude::ValidationResultExtensions::prepend_path(#first_acc, #first_label)
                                },
                                |assign, (next_acc, next_label)| {
                                    quote! { sealedstruct::prelude::ValidationResultExtensions::combine(#assign,
                                        sealedstruct::prelude::ValidationResultExtensions::prepend_path(#next_acc, #next_label))
                                    }
                                },
                            );
                            // Generates e.g.:
                            // input.0.prependPath("0").combine(input.1.prependPath("1")).combine(input.2.prepend_path("2"));
                            quote! {
                                #assign.map(|_| ())
                            }
                        }
                        _ => TokenStream::new(),
                    }
                }
                Fields::Unit => {
                    // Unit structs cannot own more than 0 bytes of heap memory.
                    quote! { Ok(())}
                }
            }
        }
        Data::Enum(ref e) => {
            let field_mappings = e.variants.iter().map(|v| {
                let ident = &v.ident;
                match &v.fields {
                    &Fields::Unit => quote! {
                        #result_name::#ident => #raw_name::#ident,
                    },
                    _ => unimplemented!("Just unit fields are supported"),
                }
            });
            quote! {
                Ok(#wrapper_name(match input {
                    #(#field_mappings)*
                }))
            }
        }
        Data::Union(_) => unimplemented!(),
    }
}

fn create_result(data: &Data, result_type: TokenStream) -> TokenStream {
    match *data {
        Data::Struct(ref data) => match data.fields {
            Fields::Named(ref fields) => {
                let recurse = fields.named.iter().map(|f| {
                    let name = &f.ident;
                    let vis = &f.vis;
                    quote_spanned! {f.span()=>
                        #vis #name: sealedstruct::Result<()>,
                    }
                });
                quote! {
                    struct #result_type {
                        #(#recurse)*
                    }
                }
            }
            Fields::Unnamed(ref fields) => {
                let recurse = fields.unnamed.iter().map(|f| {
                    let vis = &f.vis;
                    quote_spanned! {f.span()=>
                        #vis sealedstruct::Result<()>
                    }
                });
                quote! {
                    struct #result_type(#(#recurse,)*);
                }
            }

            Fields::Unit => unimplemented!("Unit-Struct not supported"),
        },
        Data::Enum(ref e) => {
            let recurse = e.variants.iter().map(|variant| match &variant.fields {
                Fields::Named(_) => unimplemented!("Enums with named fields are not supported"),
                Fields::Unnamed(_) => unimplemented!("Enums with unnamed fields are not supported"),
                Fields::Unit => {
                    let x = &variant.ident;
                    quote! {#x,}
                }
            });
            quote! {
                enum #result_type {
                    #(#recurse)*
                }
            }
        }
        Data::Union(_) => unimplemented!("Unions are not supported"),
    }
}
