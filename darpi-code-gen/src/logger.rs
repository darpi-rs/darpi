use logos;
use logos::Logos;
use proc_macro2::Span;
use quote::quote;
use syn::{Error, ExprLit, ItemStruct, Lit};

#[derive(Logos, Debug, PartialEq)]
pub enum ReqFmtTok {
    #[token("%a")]
    RemoteIP,

    #[token("%t")]
    When,

    #[token("%u")]
    Url,

    #[token("%b")]
    BodySize,

    #[regex("%[{][A-Z][a-zA-Z_-]+[}]h")]
    HeaderValue,

    #[regex("%[{][A-Z][a-zA-Z_-]+[}]e")]
    EnvValue,

    #[regex(r"[^%\t\n\fa-zA-z{}]+")]
    Sep,

    #[error]
    #[regex(r"[\t\n\f]+", logos::skip)]
    Error,
}

#[derive(Logos, Debug, PartialEq)]
pub enum RespFmtTok {
    #[token("%a")]
    RemoteIP,

    #[token("%t")]
    When,

    #[token("%T")]
    Took,

    #[token("%s")]
    Status,

    #[token("%b")]
    BodySize,

    #[regex(r"%[{][A-Z][a-zA-Z_-]+[}]h")]
    HeaderValue,

    #[regex(r"%[{][A-Z][a-zA-Z_-]+[}]e")]
    EnvValue,

    #[regex(r"[^%\t\n\fa-zA-z{}]+")]
    Sep,

    #[error]
    #[regex(r"[\t\n\f]+", logos::skip)]
    Error,
}

pub fn make_res_fmt(
    expr_lit: ExprLit,
    item_struct: ItemStruct,
) -> Result<proc_macro::TokenStream, Error> {
    if let Lit::Str(str) = expr_lit.lit {
        let val = str.value();
        let mut lex = RespFmtTok::lexer(&val);
        let mut variables = vec![quote! {let mut content = vec!["[response]".to_string()];}];

        while let Some(next) = lex.next() {
            match next {
                RespFmtTok::Error => {
                    return Err(Error::new(
                        Span::call_site(),
                        format!("invalid format value: {:#?}", lex.slice()),
                    ))
                }
                RespFmtTok::RemoteIP => variables.push(quote! {
                    let ip = if let Some(forwarded) = r.headers().get(darpi::header::FORWARDED) {
                        format!("{}",forwarded.to_str().map_err(|_| "").expect("never to happen"))
                    } else {
                        format!("unknown")
                    };
                    let forwarded = format!("remote_ip: {}",ip);
                    content.push(forwarded);
                }),
                RespFmtTok::When => {
                    variables.push(quote! {
                        let now = format!("when: {}", darpi::chrono::Utc::now());
                        content.push(now);
                    });
                }
                RespFmtTok::BodySize => {
                    variables.push(quote! {
                        let size = format!("body_size: {} byte(s)", r.size_hint().upper().unwrap_or(r.size_hint().lower()));
                        content.push(size);
                    });
                }
                RespFmtTok::HeaderValue => {
                    let variable = lex.slice();

                    variables.push(quote! {
                        if let Some(variable) = r.headers().get(#variable) {
                        let variable = format!(
                            "{}: {}",
                            #variable,
                            variable.to_str().map_err(|_| "").expect("never to happen")
                        );
                        content.push(variable);
                    }
                    });
                }
                RespFmtTok::EnvValue => {
                    let variable = lex.slice();

                    variables.push(quote! {
                        if let Ok(variable) = std::env::var(#variable) {
                            content.push(format!("{}: {}", #variable, variable));
                        }
                    });
                }
                RespFmtTok::Sep => {
                    let sep = lex.slice();

                    variables.push(quote! {
                        content.push(format!("{}", #sep));
                    });
                }
                RespFmtTok::Status => {
                    variables.push(quote! {
                        content.push(format!("[status]: {}", r.status()));
                    });
                }
                RespFmtTok::Took => {
                    variables.push(quote! {
                        content.push(format!("took: {:#?}", start.elapsed()));
                    });
                }
            }
        }

        variables.push(quote! {
            content.join(" ").into()
        });

        let name = &item_struct.ident;
        let q = quote! {
            #item_struct
            impl darpi::RespFormatter for #name {
                fn format_resp(&self, start: &std::time::Instant, r: &darpi::Response<darpi::Body>) -> String {
                    use darpi::HttpBody;
                    #(#variables )*
                }
            }
        };
        //panic!("{}", q.to_string());
        return Ok(q.into());
    }

    Err(Error::new(
        Span::call_site(),
        "only string literal is supported",
    ))
}

pub fn make_req_fmt(
    expr_lit: ExprLit,
    item_struct: ItemStruct,
) -> Result<proc_macro::TokenStream, Error> {
    if let Lit::Str(str) = expr_lit.lit {
        let val = str.value();
        let mut lex = ReqFmtTok::lexer(&val);
        let mut variables = vec![quote! {let mut content = vec!["[request]".to_string()];}];

        while let Some(next) = lex.next() {
            match next {
                ReqFmtTok::Error => {
                    return Err(Error::new(
                        Span::call_site(),
                        format!("invalid format value: {:#?}", lex.slice()),
                    ))
                }
                ReqFmtTok::RemoteIP => variables.push(quote! {
                    let ip = if let Some(forwarded) = rp.headers.get(darpi::header::FORWARDED) {
                        format!("{}",forwarded.to_str().map_err(|_| "").expect("never to happen"))
                    } else {
                        format!("unknown")
                    };
                    let forwarded = format!("remote_ip: {}",ip);
                    content.push(forwarded);
                }),
                ReqFmtTok::When => {
                    variables.push(quote! {
                        let now = format!("when: {}", darpi::chrono::Utc::now());
                        content.push(now);
                    });
                }
                ReqFmtTok::Url => {
                    variables.push(quote! {
                        let uri = format!("uri: {:#?}", rp.uri);
                        content.push(uri);
                    });
                }
                ReqFmtTok::BodySize => {
                    variables.push(quote! {
                        let size = format!("body_size: {} byte(s)", b.size_hint().upper().unwrap_or(b.size_hint().lower()));
                        content.push(size);
                    });
                }
                ReqFmtTok::HeaderValue => {
                    let variable = lex.slice();

                    variables.push(quote! {
                        if let Some(variable) = rp.headers.get(#variable) {
                        let variable = format!(
                            "{}: {}",
                            #variable,
                            variable.to_str().map_err(|_| "").expect("never to happen")
                        );
                        content.push(variable);
                    }
                    });
                }
                ReqFmtTok::EnvValue => {
                    let variable = lex.slice();

                    variables.push(quote! {
                        if let Ok(variable) = std::env::var(#variable) {
                            content.push(format!("{}: {}", #variable, variable));
                        }
                    });
                }
                ReqFmtTok::Sep => {
                    let sep = lex.slice();

                    variables.push(quote! {
                        content.push(format!("{}", #sep));
                    });
                }
            }
        }

        variables.push(quote! {
            content.join(" ").into()
        });

        let name = &item_struct.ident;
        let q = quote! {
            #item_struct
            impl darpi::ReqFormatter for #name {
                fn format_req(&self, b: &darpi::Body, rp: &darpi::RequestParts) -> String {
                    use darpi::HttpBody;
                    #(#variables )*
                }
            }
        };
        //panic!("{}", q.to_string());
        return Ok(q.into());
    }

    Err(Error::new(
        Span::call_site(),
        "only string literal is supported",
    ))
}
