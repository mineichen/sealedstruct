use proc_macro2::{Ident, Span, TokenStream};
use quote::{quote, quote_spanned, ToTokens};
use syn::spanned::Spanned;
use syn::{
    parse_macro_input, parse_quote, Data, DeriveInput, Fields, Generics, Index, TypeParamBound,
    Visibility, WhereClause,
};

pub fn derive_seal(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    // Parse the input tokens into a syntax tree.
    let input = parse_macro_input!(input as DeriveInput);
    let mut attrs = input.attrs.iter().filter(|x| match x.path().get_ident() {
        Some(ident) => ident == "sealedDerive",
        None => false,
    });
    let inner_derive = if let Some(x) = attrs.next() {
        let token_list = x
            .meta
            .require_list()
            .expect("Only supports list attributes");
        let token = &token_list.tokens;

        quote! {
            #[derive(#token)]
        }
    } else {
        TokenStream::new()
    };

    if let syn::Visibility::Inherited = input.vis {
        panic!("Raw-Struct mustn't be private. Deriving 'Seal' only makes sense if generated Sealed* is in submodule");
    }

    // Used in the quasi-quotation below as `#name`.
    let raw_name = input.ident;
    let raw_name_str = raw_name.to_string();
    if !raw_name_str.ends_with("Raw") {
        panic!("Struct name must end with 'Raw'");
    }

    let sealable_generics = add_trait_bounds(
        input.generics.clone(),
        &[parse_quote!(sealedstruct::Sealable)],
    );
    let create_inner_generics = sealable_generics.clone();
    let (impl_generics, ty_generics, where_clause) = sealable_generics.split_for_impl();

    let struct_name_str = &raw_name_str[..(raw_name_str.len() - 3)];
    let facade_name = syn::Ident::new(struct_name_str, raw_name.span());
    let wrapper_name = syn::Ident::new(&format!("{struct_name_str}Wrapper"), raw_name.span());
    let inner_name = syn::Ident::new(&format!("{struct_name_str}Inner"), raw_name.span());
    let result_name = syn::Ident::new(&format!("{struct_name_str}Result"), raw_name.span());

    // Generate an expression to sum up the heap size of each field.
    let inner = create_inner(&input.data, &inner_name, create_inner_generics, &input.vis);
    let result = create_result(
        &input.data,
        quote! { #result_name #impl_generics #where_clause},
    );
    let result_into_inner = create_result_into_inner_body(&input.data, &inner_name, &result_name);
    let inner_into_raw = create_inner_into_raw_body(&input.data, &inner_name, &raw_name);
    let cmp_body = create_cmp_raw_with_inner_body(&input.data, &raw_name, &inner_name);
    let input_vis = input.vis;

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
    };
    #[cfg(not(feature = "serde"))]
    let serde_wrapper = quote! {};

    let expanded = quote! {
        #result

        #inner_derive
        #inner

        #serde_wrapper

        #input_vis type #facade_name #ty_generics  = #wrapper_name<#inner_name #ty_generics>;

        #[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, Default)]
        pub struct #wrapper_name<T>(T);

        impl #impl_generics #facade_name #ty_generics #where_clause {
            pub fn new<TRaw: sealedstruct::TryIntoNested<Target = #inner_name #ty_generics>>(raw: TRaw) -> sealedstruct::Result<Self> {
                Ok(#wrapper_name(TRaw::try_into_nested(raw)?))
            }
            pub fn into_inner(self) -> #inner_name #ty_generics {
                self.0
            }
        }

        impl<T> std::ops::Deref for #wrapper_name<T> {
            type Target = T;

            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl #impl_generics sealedstruct::Sealable for #raw_name #ty_generics #where_clause
        {
            type Target = #facade_name #ty_generics;

            fn seal(self) -> sealedstruct::Result<Self::Target> {
                Self::Target::new(self)
            }

            fn open(sealed: Self::Target) -> Self {
                sealed.0.into()
            }

            fn partial_eq(&self, other: &Self::Target) -> bool {
                self.eq(&other.0)
            }
        }


         impl #impl_generics From<#inner_name #ty_generics> for #raw_name #ty_generics {
            fn from(input: #inner_name #ty_generics) -> Self {
                #inner_into_raw
            }
        }

        impl #impl_generics From<#result_name #ty_generics> for sealedstruct::Result<#inner_name #ty_generics> {
            fn from(input: #result_name #ty_generics) -> Self {
                #result_into_inner
            }
        }

        impl #impl_generics std::cmp::PartialEq<#inner_name #ty_generics> for #raw_name #ty_generics {
            fn eq(&self, other: & #inner_name #ty_generics ) -> bool {
                #cmp_body
            }
        }
        impl #impl_generics std::cmp::PartialEq<#facade_name #ty_generics> for #raw_name #ty_generics  {
            fn eq(&self, other: &#facade_name #ty_generics) -> bool {
                self == std::ops::Deref::deref(other)
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
            Fields::Unnamed(ref fields) => {
                (0..fields.unnamed.len()).fold(quote! { true }, |acc, f| {
                    let ident = Index::from(f);
                    quote!( #acc && sealedstruct::Sealable::partial_eq(&self.#ident, &other.#ident))
                })
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
                        Ok(#inner_name { #(#field_idents,)* })
                    }
                }
                Fields::Unnamed(ref fields) => {
                    let field_idents = (0..fields.unnamed.len())
                        .map(|f| Ident::new(&format!("x{f}"), Span::call_site()));
                    let mut ident_iter = (0..fields.unnamed.len()).map(|f| {
                        let index = Index::from(f);
                        (
                            quote! { input.#index },
                            Ident::new(&format!("x{f}"), Span::call_site()),
                            f.to_string(),
                        )
                    });
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
                        Ok(#inner_name(#(#field_idents,)* ))
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
            Fields::Unnamed(ref fields) => {
                let field_mappings = (0..fields.unnamed.len()).map(|f| {
                    let ident = Index::from(f);
                    quote! { #ident: sealedstruct::Sealable::open(input.#ident),}
                });

                quote! {
                    #raw_name { #(#field_mappings)* }
                }
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
fn create_inner(
    data: &Data,
    inner_name: &Ident,
    generics: Generics,
    vis: &Visibility,
) -> TokenStream {
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let inner_type = quote! { #inner_name #impl_generics #where_clause };
    match *data {
        Data::Struct(ref data) => match data.fields {
            Fields::Named(ref fields) => {
                let struct_fields = fields.named.iter().map(|f| {
                    let name = &f.ident;
                    let ty = &f.ty;
                    let vis = &f.vis;
                    quote_spanned! {f.span()=>
                        #vis #name: <#ty as sealedstruct::Sealable>::Target,
                    }
                });

                let cmp_where_clause =
                    build_target_where_clause(generics.clone(), parse_quote!(std::cmp::PartialEq));
                let cmp_fields = fields.named.iter().map(|f| {
                    let ident = &f.ident;
                    quote_spanned! {f.span()=>
                        && self. #ident == other. #ident
                    }
                });

                let dbg_where_clause =
                    build_target_where_clause(generics.clone(), parse_quote!(std::fmt::Debug));
                let dbg_fields = fields.named.iter().map(|f| {
                    let name_str = f.ident.as_ref().expect("Always has a ident").to_string();
                    let name = &f.ident;
                    quote_spanned! {f.span()=>
                        .field(#name_str, &self. #name)
                    }
                });

                let inner_name_str = inner_name.to_string();
                if !generics.params.is_empty() {
                    quote! {
                        // See if StructNameInner could be changed to have no generic with corresponding StructNameRaw
                        // This would allow Auto-Derive of PartialEq and Debug
                        impl #impl_generics std::cmp::PartialEq for #inner_name #ty_generics #cmp_where_clause {
                            fn eq(&self, other: &Self) -> bool {
                                true #(#cmp_fields)*
                            }
                        }

                        impl #impl_generics std::fmt::Debug for #inner_name #ty_generics #dbg_where_clause  {
                            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                                f.debug_struct(#inner_name_str) #(#dbg_fields)* .finish()
                            }
                        }
                        #vis struct #inner_type {
                            #(#struct_fields)*
                        }
                    }
                } else {
                    quote! {
                        #[derive(PartialEq, Debug)]
                        #vis struct #inner_type {
                            #(#struct_fields)*
                        }
                    }
                }
            }
            Fields::Unnamed(ref fields) => {
                let recurse = fields.unnamed.iter().map(|f| {
                    let ty = &f.ty;
                    let vis = &f.vis;
                    quote_spanned! {f.span()=>
                        #vis <#ty as sealedstruct::Sealable>::Target,
                    }
                });
                quote! {
                    #[derive(PartialEq, Debug)]
                    #vis struct #inner_type(#(#recurse)*);
                }
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
                #vis enum #inner_type {
                    #(#recurse)*
                }
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
                    let ty = &f.ty;
                    let vis = &f.vis;
                    quote_spanned! {f.span()=>
                        #vis #name: sealedstruct::Result<<#ty as sealedstruct::Sealable>::Target>,
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
                    let ty = &f.ty;
                    let vis = &f.vis;
                    quote_spanned! {f.span()=>
                        #vis sealedstruct::Result<<#ty as sealedstruct::Sealable>::Target>,
                    }
                });
                quote! {
                    struct #result_type(#(#recurse)*);
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

pub(crate) fn build_target_where_clause(
    mut generics: Generics,
    bound: TypeParamBound,
) -> WhereClause {
    let mut where_clause = generics.make_where_clause().clone();
    for type_param in &mut generics.type_params() {
        let ident = &type_param.ident;
        where_clause
            .predicates
            .push(parse_quote! {<#ident as sealedstruct::Sealable>::Target:  #bound})
    }
    where_clause
}
pub(crate) fn add_trait_bounds(mut generics: Generics, bounds: &[TypeParamBound]) -> Generics {
    for type_param in &mut generics.type_params_mut() {
        type_param.bounds.extend(bounds.iter().cloned());
    }
    generics
}
