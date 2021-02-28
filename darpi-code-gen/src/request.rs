use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, token::Pound, AttrStyle, Attribute, Error, Fields, ItemStruct, Path};

pub(crate) fn make_path_type(input: TokenStream) -> TokenStream {
    let mut struct_arg = parse_macro_input!(input as ItemStruct);
    let name = struct_arg.ident.clone();

    if let Fields::Named(ref mut named) = &mut struct_arg.fields {
        if named.named.is_empty() {
            return Error::new_spanned(named, "Empty path makes no sense")
                .to_compile_error()
                .into();
        }

        named.named.iter_mut().for_each(|f| {
            f.attrs.push(Attribute {
                pound_token: Pound::default(),
                style: AttrStyle::Outer,
                bracket_token: Default::default(),
                path: Path {
                    leading_colon: None,
                    segments: Default::default(),
                },
                tokens: quote! {
                    serde(deserialize_with = "darpi::from_str")
                },
            })
        });

        let tokens = quote! {
            #struct_arg
            impl darpi::response::ErrResponder<darpi::request::PathError, darpi::Body> for #name {
                fn respond_err(e: darpi::request::PathError) -> darpi::Response<darpi::Body> {
                    let msg = match e {
                        darpi::request::PathError::Deserialize(msg) => msg,
                    };

                    darpi::Response::builder()
                        .status(darpi::StatusCode::BAD_REQUEST)
                        .body(darpi::Body::from(msg))
                        .expect("this not to happen!")
                }
            }
        };

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
