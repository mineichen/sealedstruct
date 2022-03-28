use proc_macro2::{Ident, TokenStream};
use quote::{quote, quote_spanned, ToTokens};
use syn::spanned::Spanned;
use syn::{parse_macro_input, Data, DeriveInput, Fields};

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
    let inner_name = syn::Ident::new(&format!("{struct_name_str}Inner"), raw_name.span());
    let sealed_name = syn::Ident::new(&format!("{struct_name_str}Sealed"), raw_name.span());
    let result_name = syn::Ident::new(&format!("{struct_name_str}Result"), raw_name.span());

    // Generate an expression to sum up the heap size of each field.
    let inner = create_inner(&input.data, &inner_name);
    let result = create_result_fields(&input.data, &result_name);
    let result_into_sealed =
        create_result_into_sealed_body(&input.data, &sealed_name, &inner_name, &result_name);
    let sealed_into_raw = create_sealed_into_raw_body(&input.data, &inner_name, &raw_name);
    let cmp_body = create_cmp_raw_with_sealed_body(&input.data, &raw_name, &inner_name);

    let expanded = quote! {
        #result
        #inner

        #[derive(PartialEq, Debug)]
        pub struct #sealed_name( #inner_name );

        impl std::ops::Deref for #sealed_name {
            type Target = #inner_name;

            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl From<#sealed_name> for #raw_name {
            fn from(input: #sealed_name) -> Self {
                #sealed_into_raw
            }
        }


        impl From<#result_name> for sealedstruct::Result<#sealed_name> {
            fn from(input: #result_name) -> Self {
                #result_into_sealed
            }
        }

        impl std::cmp::PartialEq<#sealed_name> for #raw_name {
            fn eq(&self, other: & #sealed_name ) -> bool {
                #cmp_body
            }
        }
    };

    // Hand the output tokens back to the compiler.
    proc_macro::TokenStream::from(expanded)
}

fn create_cmp_raw_with_sealed_body(
    data: &Data,
    raw_name: &Ident,
    inner_name: &Ident,
) -> TokenStream {
    match *data {
        Data::Struct(ref data) => match data.fields {
            Fields::Named(ref fields) => fields.named.iter().fold(quote! { true }, |acc, f| {
                let ident = &f.ident;
                quote!( #acc && self.#ident == other.#ident )
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
                            &#raw_name::#ident => &other.0 == &#inner_name::#ident,
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

fn create_result_into_sealed_body(
    data: &Data,
    sealed_name: &Ident,
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
                        Ok(#sealed_name(#inner_name { #(#field_idents,)* }))
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
                Ok(#sealed_name(match input {
                    #(#field_mappings)*
                }))
            }
        }
        Data::Union(_) => unimplemented!(),
    }
}
fn create_sealed_into_raw_body(data: &Data, inner_name: &Ident, raw_name: &Ident) -> TokenStream {
    match *data {
        Data::Struct(ref data) => match data.fields {
            Fields::Named(ref fields) => {
                let field_mappings = fields.named.iter().map(|f| {
                    let ident = &f.ident;
                    quote! { #ident: input.#ident.into(),}
                });

                quote! {
                    let input = input.0;
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
                match input.0 {
                    #(#field_mappings)*
                }
            }
        }
        Data::Union(_) => unimplemented!(),
    }
}
fn create_inner(data: &Data, inner_name: &Ident) -> TokenStream {
    match *data {
        Data::Struct(ref data) => match data.fields {
            Fields::Named(ref fields) => {
                let recurse = fields.named.iter().map(|f| {
                    let name = &f.ident;
                    let ty = &f.ty;
                    quote_spanned! {f.span()=>
                        pub #name: <#ty as sealedstruct::TryIntoSealed>::Target,
                    }
                });
                quote! {
                    #[derive(PartialEq, Debug)]
                    pub struct #inner_name {
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
                pub enum #inner_name {
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
                            #vis #name: sealedstruct::Result<<#ty as sealedstruct::TryIntoSealed>::Target>,
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
                Fields::Named(x) => unimplemented!("Enums with named fields are not supported"),
                Fields::Unnamed(x) => unimplemented!("Enums with unnamed fields are not supported"),
                Fields::Unit => {
                    let x = &variant.ident;
                    quote! {#x,}
                }
            });
            quote! {
                pub enum #result_name {
                    #(#recurse)*
                }
            }
        }
        Data::Union(_) => unimplemented!("Unions are not supported"),
    }
}
