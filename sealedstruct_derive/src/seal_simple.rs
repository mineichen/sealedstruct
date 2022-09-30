use proc_macro2::{Ident, TokenStream, Span};
use quote::{quote, quote_spanned, ToTokens};
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

    let struct_name_str = &raw_name_str[..(raw_name_str.len() - 3)];
    let wrapper_name = syn::Ident::new(&format!("{struct_name_str}"), raw_name.span());
    let result_name = syn::Ident::new(&format!("{struct_name_str}Result"), raw_name.span());

    // Generate an expression to sum up the heap size of each field.
    let result = create_result_fields(&input.data, &result_name);
    let result_into_wrapper = create_result_into_wrapper_body(&input.data, &wrapper_name, &raw_name, &result_name);
    
    #[cfg(feature = "serde")]
    let serde_wrapper = quote! {
        impl<T: serde::Serialize> serde::Serialize for #wrapper_name<T> {
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
    };
    #[cfg(not(feature = "serde"))]
    let serde_wrapper = quote! {};

    let expanded = quote! {
        #result
        
        #serde_wrapper

        impl<T: sealedstruct::Validator + From<#raw_name>> TryFrom<#raw_name> for #wrapper_name<T> {
            type Error = sealedstruct::ValidationErrors;
        
            fn try_from(value: #raw_name) -> Result<Self, Self::Error> {
                sealedstruct::Validator::check(&value)?;
                Ok(#wrapper_name(value.into()))
            }
        }

        impl #raw_name {
            pub fn seal(self) -> sealedstruct::Result<#wrapper_name> {
                self.try_into()
            }
        }

        #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Default, sealedstruct::IntoSealed)]
        pub struct #wrapper_name<T=#raw_name>(T);

        impl<T> std::ops::Deref for #wrapper_name<T> {
            type Target = T;
        
            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }
        
        impl From<#result_name> for sealedstruct::Result<#wrapper_name> {
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
                    let field_idents = fields
                        .named
                        .iter()
                        .map(|f| f.ident.clone().into_token_stream());
                    let mut ident_iter = fields.named.iter().flat_map(|f| f.ident.clone());
                    let field_list = match ident_iter.next() {
                        Some(first) => {
                            let first_string = first.to_string();
                            let (fields, assign) = ident_iter.fold(
                                (first.to_token_stream(), quote!{
                                    sealedstruct::prelude::ValidationResultExtensions::prepend_path(input.#first, #first_string)
                                }),
                                |(fields_list, assign), next| {
                                    let next_text = next.to_string(); 
                                    (
                                        quote! {(#fields_list, #next)},
                                        quote! { sealedstruct::prelude::ValidationResultExtensions::combine(#assign, 
                                            sealedstruct::prelude::ValidationResultExtensions::prepend_path(input.#next, #next_text)) 
                                        },
                                    )
                                },
                            );
                            // Generates e.g.:
                            // let ((foo, bar), baz) = input.foo.combine(input.bar).combine(input.baz)?;
                            quote! {
                                let #fields = #assign?;
                            }
                        }
                        _ => TokenStream::new(),
                    };
                    quote! {
                        #field_list
                        Ok(#wrapper_name(#raw_name { #(#field_idents,)* }))
                    }
                }
                Fields::Unnamed(ref fields) => {
                    let field_idents =(0..fields.unnamed.len())
                        .map(|f| Ident::new(&format!("x{f}"), Span::call_site()));
                    let mut ident_iter = (0..fields.unnamed.len()).map(|f|{
                        let index = Index::from(f); (
                        quote! { input.#index },
                        Ident::new(&format!("x{f}"), Span::call_site()),
                        f.to_string()
                    )});
                    let field_list = match ident_iter.next() {
                        Some((first_acc, first_var, first_label)) => {
                            let (fields, assign) = ident_iter.fold(
                                (first_var.to_token_stream(), quote!{
                                    sealedstruct::prelude::ValidationResultExtensions::prepend_path(#first_acc, #first_label)
                                }),
                                |(fields_list, assign), (next_acc, next_var, next_label)| {
                                    (
                                        quote! {(#fields_list, #next_var)},
                                        quote! { sealedstruct::prelude::ValidationResultExtensions::combine(#assign, 
                                            sealedstruct::prelude::ValidationResultExtensions::prepend_path(#next_acc, #next_label)) 
                                        },
                                    )
                                },
                            );
                            // Generates e.g.:
                            // let ((foo, bar), baz) = input.foo.combine(input.bar).combine(input.baz)?;
                            quote! {
                                let #fields = #assign?;
                            }
                        }
                        _ => TokenStream::new(),
                    };
                    quote! {
                        #field_list
                        Ok(#wrapper_name(#raw_name(#(#field_idents,)* ))) 
                    }
                }
                Fields::Unit => {
                    // Unit structs cannot own more than 0 bytes of heap memory.
                    TokenStream::new()
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

fn create_result_fields(data: &Data, result_name: &Ident) -> TokenStream {
    match *data {
        Data::Struct(ref data) => match data.fields {
            Fields::Named(ref fields) => {
                let recurse = fields.named.iter().map(|f| {
                    let name = &f.ident;
                    let ty = &f.ty;
                    let vis = &f.vis;
                    quote_spanned! {f.span()=>
                        #vis #name: sealedstruct::Result<<#ty as sealedstruct::Sealable>::Target>,
                    }
                });
                quote! {
                    struct #result_name {
                        #(#recurse)*
                    }
                }
            }
            Fields::Unnamed(ref fields) => {
                let recurse = fields.unnamed.iter().map(|f| {
                    let ty = &f.ty;
                    let vis = &f.vis;
                    quote_spanned! {f.span()=>
                        #vis sealedstruct::Result<<#ty as sealedstruct::Sealable>::Target>
                    }
                });
                quote! {
                    struct #result_name(#(#recurse,)*);
                }
            },

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
                enum #result_name {
                    #(#recurse)*
                }
            }
        }
        Data::Union(_) => unimplemented!("Unions are not supported"),
    }
}
