use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use syn::{parse_macro_input, Error, Fields, ItemStruct};

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
        let mut strings = vec![];

        let mut i = 0;
        let mut sorted_fields = Vec::with_capacity(named.named.len());

        for field in named.named.iter() {
            if let Some(name) = &field.ident {
                sorted_fields.push((name.clone(), field.ty.clone()));
                continue;
            }

            return Error::new_spanned(field, "Field should have a name")
                .to_compile_error()
                .into();
        }

        sorted_fields.sort_by(|a, b| a.0.cmp(&b.0));

        for field in sorted_fields {
            let name = field.0;
            let ttype = &field.1;
            let index = syn::Index::from(i);
            let q = quote! {
                let #name: #ttype = match std::str::FromStr::from_str(&args.#index) {
                    Ok(k) => k,
                    Err(e) => return Err(darpi::request::PathError::Deserialize(e.to_string()))
                };
            };

            fields_create.push(q);
            fields.push(name.to_token_stream());
            strings.push(quote! {String});
            i += 1;
        }

        let tokens = quote! {
            impl std::convert::TryFrom<(#(#strings ,)*)> for #name {
                type Error = darpi::request::PathError;

                fn try_from(args: (#(#strings ,)*)) -> Result<Self, Self::Error> {
                    #(#fields_create)*
                    Ok(Self{#(#fields ,)*})
                }
            }

            impl darpi::response::ErrResponder<darpi::request::PathError, darpi::Body> for #name {
                #[cold]
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
            #[cold]
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
