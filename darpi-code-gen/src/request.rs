use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::{quote, ToTokens};
use syn::{parse_macro_input, Error, Fields, ItemStruct, LitStr};

pub(crate) fn make_path_type(input: TokenStream) -> TokenStream {
    let mut struct_arg = parse_macro_input!(input as ItemStruct);
    let name = struct_arg.ident.clone();

    if let Fields::Named(ref mut named) = &mut struct_arg.fields {
        if named.named.is_empty() {
            return Error::new_spanned(named, "Empty path makes no sense")
                .to_compile_error()
                .into();
        }

        let mut fields_create = vec![];
        let mut fields = vec![];

        for field in named.named.iter() {
            if let Some(name) = &field.ident {
                let name_str = LitStr::new(&name.to_string(), Span::call_site());
                let ttype = &field.ty;
                let q = quote! {
                    let #name = match args.get(#name_str) {
                        Some(n) => *n,
                        None => return Err(darpi::request::PathError::Missing(#name_str.into()))
                    };

                    let #name = match #ttype::try_from(#name) {
                        Ok(k) => k,
                        Err(e) => return Err(darpi::request::PathError::Deserialize(e.to_string()))
                    };
                };
                fields_create.push(q);
                fields.push(name.to_token_stream());
                continue;
            }

            return Error::new_spanned(field, "Field should have a name")
                .to_compile_error()
                .into();
        }

        let tokens = quote! {
            impl<'a> std::convert::TryFrom<std::collections::HashMap<&'a str, &'a str>> for #name {
                type Error = darpi::request::PathError;

                fn try_from(args: std::collections::HashMap<&'a str, &'a str, std::collections::hash_map::RandomState>) -> Result<Self, Self::Error> {
                    #(#fields_create)*
                    Ok(Self{#(#fields ,)*})
                }
            }

            impl darpi::response::ErrResponder<darpi::request::PathError, darpi::Body> for #name {
                fn respond_err(e: darpi::request::PathError) -> darpi::Response<darpi::Body> {
                    let msg = match e {
                        darpi::request::PathError::Deserialize(msg) => msg,
                        darpi::request::PathError::Missing(msg) => msg,
                    };

                    darpi::Response::builder()
                        .status(darpi::StatusCode::BAD_REQUEST)
                        .body(darpi::Body::from(msg))
                        .expect("this not to happen!")
                }
            }
        };
        //panic!("{}", tokens.to_token_stream());
        return tokens.into();
    }
    Error::new_spanned(struct_arg, "Tuple structs not supported")
        .to_compile_error()
        .into()
}

pub(crate) fn make_query_type(input: TokenStream) -> TokenStream {
    let struct_arg = parse_macro_input!(input as ItemStruct);
    let name = &struct_arg.ident;

    let tokens = quote! {
        impl darpi::response::ErrResponder<darpi::request::QueryPayloadError, darpi::Body> for #name {
            fn respond_err(e: darpi::request::QueryPayloadError) -> darpi::Response<darpi::Body> {
                let msg = match e {
                    darpi::request::QueryPayloadError::Deserialize(de) => de.to_string(),
                    darpi::request::QueryPayloadError::NotExist => "missing query params".to_string(),
                };

                darpi::Response::builder()
                    .status(darpi::StatusCode::BAD_REQUEST)
                    .body(darpi::Body::from(msg))
                    .expect("this not to happen!")
            }
        }
    };
    tokens.into()
}
