use proc_macro2::{Ident, TokenStream};
use quote::{quote, quote_spanned, ToTokens};
use syn::spanned::Spanned;
use syn::{parse_macro_input, Data, DeriveInput, Fields, Visibility};

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
    let inner_name = syn::Ident::new(struct_name_str, raw_name.span());
    let result_name = syn::Ident::new(&format!("{struct_name_str}Result"), raw_name.span());

    // Generate an expression to sum up the heap size of each field.
    let inner = create_inner(&input.data, &inner_name, input.vis);
    let result = create_result_fields(&input.data, &result_name);
    let result_into_inner = create_result_into_inner_body(&input.data, &inner_name, &result_name);
    let inner_into_raw = create_inner_into_raw_body(&input.data, &inner_name, &raw_name);
    let cmp_body = create_cmp_raw_with_inner_body(&input.data, &raw_name, &inner_name);

    let expanded = quote! {
        #result
        #inner

        impl From<#inner_name> for #raw_name {
            fn from(input: #inner_name) -> Self {
                #inner_into_raw
            }
        }

        impl From<#result_name> for sealedstruct::Result<#inner_name> {
            fn from(input: #result_name) -> Self {
                #result_into_inner
            }
        }

        impl std::cmp::PartialEq<#inner_name> for #raw_name {
            fn eq(&self, other: & #inner_name ) -> bool {
                #cmp_body
            }
        }
        impl std::cmp::PartialEq<sealedstruct::Sealed<#inner_name>> for #raw_name {
            fn eq(&self, other: &sealedstruct::Sealed<#inner_name>) -> bool {
                let other: & #inner_name = other;
                self == other
            }
        }
    };

    // Hand the output tokens back to the compiler.
    proc_macro::TokenStream::from(expanded)
}

fn create_cmp_raw_with_inner_body(
    data: &Data,
    raw_name: &Ident,
    inner_name: &Ident,
) -> TokenStream {
    match *data {
        Data::Struct(ref data) => match data.fields {
            Fields::Named(ref fields) => fields.named.iter().fold(quote! { true }, |acc, f| {
                let ident = &f.ident;
                quote!( #acc && sealedstruct::Sealable::partial_eq(&self.#ident, &other.#ident))
            }),
            Fields::Unnamed(ref _fields) => {
                unimplemented!("Tuple-Structs are not supported yet");
            }
            Fields::Unit => TokenStream::new(),
        },
        Data::Enum(ref e) => {
            let field_mappings = e.variants.iter().map(|v| {
                let ident = &v.ident;
                match &v.fields {
                    &Fields::Unit => {
                        quote! {
                            &#raw_name::#ident => &other == &&#inner_name::#ident,
                        }
                    }
                    _ => unimplemented!("Just unit fields are supported"),
                }
            });

            quote! {
                match self {
                    #(#field_mappings)*
                }
            }
        } // Todo: Wrong, but compiles...
        Data::Union(_) => unimplemented!(),
    }
}

fn create_result_into_inner_body(
    data: &Data,
    inner_name: &Ident,
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
                    let mut ident_iter = fields.named.iter().map(|f| f.ident.clone());
                    let field_list = match ident_iter.next() {
                        Some(first) => {
                            let (fields, assign) = ident_iter.fold(
                                (first.clone().into_token_stream(), quote!(input.#first)),
                                |(fields_list, assign), next| {
                                    (
                                        quote! {(#fields_list, #next)},
                                        quote! { sealedstruct::prelude::ValidationResultExtensions::combine(#assign, input.#next) },
                                    )
                                },
                            );
                            // Generates e.g.:
                            // let ((foo, bar), baz) = input.foo.combine(input.bar).combine(input.baz)?;
                            quote! {
                                let #fields = #assign?;
                            }
                        }
                        None => TokenStream::new(),
                    };
                    quote! {
                        #field_list
                        Ok(#inner_name { #(#field_idents,)* })
                    }
                }
                Fields::Unnamed(ref _fields) => {
                    unimplemented!("Tuple-Structs are not supported yet");
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
                        #result_name::#ident => #inner_name::#ident,
                    },
                    _ => unimplemented!("Just unit fields are supported"),
                }
            });
            quote! {
                Ok(match input {
                    #(#field_mappings)*
                })
            }
        }
        Data::Union(_) => unimplemented!(),
    }
}
fn create_inner_into_raw_body(data: &Data, inner_name: &Ident, raw_name: &Ident) -> TokenStream {
    match *data {
        Data::Struct(ref data) => match data.fields {
            Fields::Named(ref fields) => {
                let field_mappings = fields.named.iter().map(|f| {
                    let ident = &f.ident;
                    quote! { #ident: sealedstruct::Sealable::open(input.#ident),}
                });

                quote! {
                    #raw_name { #(#field_mappings)* }
                }
            }
            Fields::Unnamed(ref _fields) => {
                unimplemented!("Tuple-Structs are not supported yet");
            }
            Fields::Unit => {
                unimplemented!("Unit-Structs are not supported yet");
            }
        },
        Data::Enum(ref e) => {
            let field_mappings = e.variants.iter().map(|v| {
                let ident = &v.ident;
                match &v.fields {
                    &Fields::Unit => quote! {
                        #inner_name::#ident => #raw_name::#ident,
                    },
                    _ => unimplemented!("Just unit fields are supported"),
                }
            });
            quote! {
                match input {
                    #(#field_mappings)*
                }
            }
        }
        Data::Union(_) => unimplemented!(),
    }
}
fn create_inner(data: &Data, inner_name: &Ident, vis: Visibility) -> TokenStream {
    match *data {
        Data::Struct(ref data) => match data.fields {
            Fields::Named(ref fields) => {
                let recurse = fields.named.iter().map(|f| {
                    let name = &f.ident;
                    let ty = &f.ty;
                    let vis = &f.vis;
                    quote_spanned! {f.span()=>
                        #vis #name: <#ty as sealedstruct::Sealable>::Target,
                    }
                });
                quote! {
                    #[derive(PartialEq, Debug)]
                    #vis struct #inner_name {
                        #(#recurse)*
                    }
                }
            }
            Fields::Unnamed(ref _fields) => {
                unimplemented!("Tuple-Structs are not supported yet");
            }
            Fields::Unit => unimplemented!(),
        },
        Data::Enum(ref e) => {
            let recurse = e.variants.iter().map(|variant| match &variant.fields {
                Fields::Named(_x) => {
                    // let recurse = x.named.iter().map(|f| quote!());
                    // quote!(#(#recurse)*);
                    unimplemented!("Named enum fields are not supported");
                }
                Fields::Unnamed(_x) => unimplemented!("Unnamed enum fields are not supported"),
                Fields::Unit => {
                    let x = &variant.ident;
                    quote! {#x,}
                }
            });
            quote! {
                #[derive(PartialEq, Debug)]
                #vis enum #inner_name {
                    #(#recurse)*
                }
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
            Fields::Unnamed(ref _fields) => unimplemented!("Tuple-Structs are not supported yet"),

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
