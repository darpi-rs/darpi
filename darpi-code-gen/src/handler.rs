use crate::app::{Func, ReqResArray};
use proc_macro::TokenStream;
use proc_macro2::{Ident, Span, TokenStream as TokenStream2};
use quote::ToTokens;
use quote::{format_ident, quote};
use std::collections::HashMap;
use syn::parse::Parse;
use syn::punctuated::Punctuated;
use syn::{
    braced, parse::ParseStream, parse_macro_input, token, Error, Expr, ExprLit, FnArg, ItemFn,
    PatType, Path, PathSegment, Result as SynResult, Type, TypePath,
};

pub(crate) const HAS_PATH_ARGS_PREFIX: &str = "HasPathArgs";
pub(crate) const HAS_NO_PATH_ARGS_PREFIX: &str = "HasNoPathArgs";
pub(crate) const MODULE_PREFIX: &str = "module";

pub(crate) fn make_handler(args: TokenStream, input: TokenStream) -> TokenStream {
    let mut func = parse_macro_input!(input as ItemFn);
    if func.sig.asyncness.is_none() {
        return Error::new_spanned(func, "Only Async functions can be used as handlers")
            .to_compile_error()
            .into();
    }

    let func_name = &func.sig.ident;
    let module_ident = quote! {args.container.clone()};
    let mut make_args = vec![];
    let mut give_args = vec![];
    let has_path_args = format_ident!("{}_{}", HAS_PATH_ARGS_PREFIX, func_name);
    let has_no_path_args = format_ident!("{}_{}", HAS_NO_PATH_ARGS_PREFIX, func_name);
    let mut map = HashMap::new();
    let mut max_middleware_index = None;
    let mut dummy_t = quote! {,T};
    let mut module_type = quote! {T};

    let args = if args.is_empty() {
        None
    } else {
        let args = parse_macro_input!(args as Config);
        Some(args)
    };
    let ArgsCall {
        mut middleware_call,
        job_call,
        container,
    } = make_args_call(args);

    if let Some(m) = container {
        dummy_t = Default::default();
        module_type = m.to_token_stream();
    }

    let mut i = 0_u32;
    let mut allowed_query = true;
    let mut allowed_path = true;
    let mut allowed_body = true;
    let mut last_args = vec![];

    for arg in func.sig.inputs.iter_mut() {
        if let FnArg::Typed(tp) = arg {
            let h_args = match make_handler_args(
                tp,
                i,
                module_ident.clone(),
                middleware_call.req_len,
                middleware_call.res_len,
            ) {
                Ok(k) => k,
                Err(e) => return e,
            };
            match h_args {
                HandlerArgs::JobChan(i, ts) => {
                    make_args.push(ts);
                    give_args.push(quote! {#i});
                }
                HandlerArgs::Query(i, ts) => {
                    if !allowed_query {
                        return Error::new_spanned(arg, "One 1 query type is allowed")
                            .to_compile_error()
                            .into();
                    }
                    allowed_query = false;
                    make_args.push(ts);
                    give_args.push(quote! {#i});
                }
                HandlerArgs::Parts(i, ts) => {
                    last_args.push(ts);
                    give_args.push(quote! {#i});
                }
                HandlerArgs::Body(i, ts) => {
                    if !allowed_body {
                        return Error::new_spanned(arg, "One 1 body type is allowed")
                            .to_compile_error()
                            .into();
                    }
                    allowed_body = false;
                    last_args.push(ts);
                    give_args.push(quote! {#i});
                }
                HandlerArgs::Path(i, ts) => {
                    if !allowed_path {
                        return Error::new_spanned(arg, "One 1 body type is allowed")
                            .to_compile_error()
                            .into();
                    }
                    allowed_path = false;
                    make_args.push(ts);
                    give_args.push(quote! {#i});
                }
                HandlerArgs::Module(i, ts) => {
                    if !dummy_t.is_empty() {
                        return Error::new_spanned(
                            arg,
                            "inject requires a container to be passed in",
                        )
                        .to_compile_error()
                        .into();
                    }
                    make_args.push(ts);
                    give_args.push(quote! {#i});
                }
                HandlerArgs::Middleware(i, ts, index, ttype) => {
                    if let Some(s) = max_middleware_index {
                        if index > s {
                            max_middleware_index = Some(index);
                        }
                    } else {
                        max_middleware_index = Some(index)
                    }
                    map.insert(index, ttype);
                    make_args.push(ts);
                    give_args.push(quote! {#i});
                }
            };

            i += 1;
            tp.attrs = Default::default();
        }
    }

    make_args.push(quote! {let (parts, body) = args.request.into_parts();});
    make_args.append(&mut last_args);

    middleware_call.req.sort_by(|a, b| a.0.cmp(&b.0));
    middleware_call.res.sort_by(|a, b| a.0.cmp(&b.0));

    let middleware_req: Vec<proc_macro2::TokenStream> =
        middleware_call.req.into_iter().map(|e| e.1).collect();
    let middleware_res: Vec<proc_macro2::TokenStream> =
        middleware_call.res.into_iter().map(|e| e.1).collect();

    let func_copy = func.clone();

    let dummy_where = if dummy_t.is_empty() {
        quote! {}
    } else {
        quote! { where T: 'static + Send + Sync}
    };

    let jobs_req = job_call.req;
    let jobs_res = job_call.res;

    let output = quote! {
        #[allow(non_camel_case_types, missing_docs)]
        trait #has_path_args {}
        #[allow(non_camel_case_types, missing_docs)]
        trait #has_no_path_args {}
        #[allow(non_camel_case_types, missing_docs)]
        pub struct #func_name;
        impl #func_name {
           #func_copy
        }

        #[darpi::async_trait]
        impl<'a #dummy_t> darpi::Handler<'a, #module_type> for #func_name #dummy_where {
            async fn call(self, mut args: darpi::Args<'a, #module_type>) -> Result<darpi::Response<darpi::Body>, std::convert::Infallible> {
               use darpi::response::Responder;
               #[allow(unused_imports)]
               use shaku::HasComponent;
               #[allow(unused_imports)]
               use darpi::request::FromQuery;
               use darpi::request::FromRequestBodyWithContainer;
               use darpi::response::ResponderError;
               #[allow(unused_imports)]
               use darpi::RequestMiddleware;
               #[allow(unused_imports)]
               use darpi::ResponseMiddleware;
               use darpi::{RequestJobFactory, ResponseJobFactory};
               #[allow(unused_imports)]
               use std::convert::TryFrom;

                #(#middleware_req )*
                #(#jobs_req )*

               #(#make_args )*

               let mut rb = Self::#func_name(#(#give_args ,)*).await.respond();

               #(#middleware_res )*
               #(#jobs_res )*

                Ok(rb)
            }
        }
    };
    //panic!("{}", output.to_string());
    output.into()
}

struct ArgsCall {
    middleware_call: MiddlewareCall,
    job_call: JobCall,
    container: Option<Path>,
}

fn make_args_call(conf: Option<Config>) -> ArgsCall {
    let (middleware, job, container) = if let Some(args) = conf {
        (args.middleware, args.jobs, args.container)
    } else {
        (None, None, None)
    };

    let middleware_call = make_call_middleware(middleware);
    let job_call = make_job_call(job);
    ArgsCall {
        middleware_call,
        job_call,
        container,
    }
}

fn get_req_middleware_arg(
    e: &Func,
    sorter: &mut u16,
    m_len: usize,
) -> Vec<proc_macro2::TokenStream> {
    let m_args: Vec<proc_macro2::TokenStream> = e
        .get_args()
        .iter()
        .map(|arg| {
            if let Expr::Call(expr_call) = arg {
                let arg_name = expr_call.func.to_token_stream().to_string();
                if arg_name == "request" {
                    let index: u16 = expr_call
                        .args
                        .first()
                        .expect("missing middleware expr")
                        .to_token_stream()
                        .to_string()
                        .parse()
                        .expect("missing middleware expr");

                    if index as usize >= m_len {
                        panic!("middleware index out of bounds");
                    }

                    *sorter += index;
                    let i_ident = format_ident!("m_arg_{}", index);
                    return quote! {#i_ident.clone()};
                } else if arg_name == "response" {
                    panic!("request middleware is executed before response middleware. therefore, it cannot ask for response middleware results")
                }
            }
            quote! {#arg}
        })
        .collect();
    m_args
}

fn get_res_middleware_arg(
    e: &Func,
    sorter: &mut u16,
    m_len: usize,
    other_len: usize,
) -> Vec<proc_macro2::TokenStream> {
    let m_args: Vec<proc_macro2::TokenStream> = e
        .get_args()
        .iter()
        .map(|arg| {
            if let Expr::Call(expr_call) = arg {
                if expr_call.func.clone().to_token_stream().to_string() == "response" {
                    let index: u16 = expr_call
                        .args
                        .first()
                        .expect("missing res middleware expr")
                        .to_token_stream()
                        .to_string()
                        .parse()
                        .expect("missing res middleware expr");

                    if index as usize >= m_len {
                        panic!("middleware index out of bounds");
                    }

                    *sorter += index;
                    let i_ident = format_ident!("m_arg_{}", index);
                    return quote! {#i_ident.clone()};
                } else if expr_call.func.to_token_stream().to_string() == "request" {
                    let index: u16 = expr_call
                        .args
                        .first()
                        .expect("missing res middleware request expr")
                        .to_token_stream()
                        .to_string()
                        .parse()
                        .expect("missing res middleware request expr");

                    if index as usize >= other_len {
                        panic!("middleware index out of bounds");
                    }

                    *sorter += index;
                    let i_ident = format_ident!("m_arg_{}", index);
                    return quote! {#i_ident.clone()};
                }
            }
            quote! {#arg}
        })
        .collect();
    m_args
}

fn make_query(
    arg_name: &Ident,
    format: Punctuated<Ident, token::Colon2>,
    full: TypePath,
) -> proc_macro2::TokenStream {
    let inner = full.path.segments.last().cloned().expect("No query");
    quote! {
        let #arg_name: #full = match #format::from_query(args.request.uri().query()) {
            Ok(q) => q,
            Err(e) => return Ok(darpi::request::assert_respond_err::<#inner, darpi::request::QueryPayloadError>(e))
        };
    }
}

fn make_path_args(arg_name: &Ident, last: &PathSegment) -> proc_macro2::TokenStream {
    quote! {
        let #arg_name = match #last::try_from(args.route_args) {
            Ok(k) => k,
            Err(e) => {
                return Ok(darpi::request::assert_respond_err::<#last, darpi::request::PathError>(
                    darpi::request::PathError::Deserialize(e.to_string()),
                ))
            }
        };
    }
}

fn make_json_body(
    arg_name: &Ident,
    path: &TypePath,
    module_ident: &TokenStream2,
) -> proc_macro2::TokenStream {
    let mut format = path.path.segments.clone();
    format
        .iter_mut()
        .for_each(|s| s.arguments = Default::default());

    let inner = &path.path.segments.last().expect("no body").arguments;

    let output = quote! {
        match #format::#inner::assert_content_type(parts.headers.get("content-type"), #module_ident).await {
            Ok(()) => {}
            Err(e) => return Ok(e.respond_err()),
        }

        let #arg_name: #path = match #format::extract(&parts.headers, body, #module_ident).await {
            Ok(q) => q,
            Err(e) => return Ok(e.respond_err())
        };
    };
    output
}

enum HandlerArgs {
    Query(Ident, proc_macro2::TokenStream),
    Body(Ident, proc_macro2::TokenStream),
    Path(Ident, proc_macro2::TokenStream),
    Module(Ident, proc_macro2::TokenStream),
    Middleware(Ident, proc_macro2::TokenStream, u64, Type),
    JobChan(Ident, proc_macro2::TokenStream),
    Parts(Ident, proc_macro2::TokenStream),
}

fn make_handler_args(
    tp: &PatType,
    i: u32,
    module_ident: proc_macro2::TokenStream,
    req_len: usize,
    _res_len: usize,
) -> Result<HandlerArgs, TokenStream> {
    let ttype = &tp.ty;

    let arg_name = format_ident!("arg_{:x}", i);
    if tp.attrs.len() != 1 {
        return Err(Error::new_spanned(
            tp,
            format!("each argument should have one attribute to define its provider"),
        )
        .to_compile_error()
        .into());
    }

    let attr = tp.attrs.first().expect("can't happen");

    if let Type::Reference(tp) = *ttype.clone() {
        if let Type::Path(_) = *tp.elem.clone() {
            let attr_ident: Vec<Ident> =
                attr.path.segments.iter().map(|s| s.ident.clone()).collect();

            if attr_ident.is_empty() {
                return Err(Error::new_spanned(attr, format!("expected an attribute"))
                    .to_compile_error()
                    .into());
            }

            if attr_ident.len() == 1 {
                let attr_ident = &attr_ident[0];

                if attr_ident == "request_parts" {
                    let res = quote! {let #arg_name = &parts;};
                    return Ok(HandlerArgs::Parts(arg_name, res));
                }
            }
        }
    }

    if let Type::Path(tp) = *ttype.clone() {
        let last = tp
            .path
            .segments
            .last()
            .expect("no handler last path segment");

        let attr_ident: Vec<Ident> = attr.path.segments.iter().map(|s| s.ident.clone()).collect();

        if attr_ident.is_empty() {
            return Err(Error::new_spanned(attr, format!("expected an attribute"))
                .to_compile_error()
                .into());
        }

        if attr_ident.len() == 1 {
            let attr_ident = &attr_ident[0];

            if attr_ident == "request" {
                let res = quote! {let #arg_name = args.request;};
                return Ok(HandlerArgs::JobChan(arg_name, res));
            }

            if attr_ident == "query" {
                let query_ttype: Punctuated<Ident, token::Colon2> =
                    tp.path.segments.iter().map(|s| s.ident.clone()).collect();

                let res = make_query(&arg_name, query_ttype, tp);
                return Ok(HandlerArgs::Query(arg_name, res));
            }

            if attr_ident == "body" {
                let res = make_json_body(&arg_name, &tp, &module_ident);
                return Ok(HandlerArgs::Body(arg_name, res));
            }

            if attr_ident == "path" {
                let res = make_path_args(&arg_name, &last);
                return Ok(HandlerArgs::Path(arg_name, res));
            }

            if attr_ident == "inject" {
                let method_resolve = quote! {
                    let #arg_name: #ttype = #module_ident.resolve();
                };
                return Ok(HandlerArgs::Module(arg_name, method_resolve));
            }
        }

        if attr_ident.len() == 2 {
            let left = &attr_ident[0];
            let right = &attr_ident[1];

            if left == "middleware" && right == "request" {
                let index: ExprLit = match attr.parse_args() {
                    Ok(el) => el,
                    Err(_) => {
                        return Err(Error::new(Span::call_site(), format!("missing index"))
                            .to_compile_error()
                            .into())
                    }
                };

                let index = match index.lit {
                    syn::Lit::Int(i) => {
                        let value = match i.base10_parse::<u64>() {
                            Ok(k) => k,
                            Err(_) => {
                                return Err(Error::new(
                                    Span::call_site(),
                                    format!("invalid middleware::request index"),
                                )
                                .to_compile_error()
                                .into())
                            }
                        };
                        value
                    }
                    _ => {
                        return Err(Error::new(
                            Span::call_site(),
                            format!("invalid middleware::request index"),
                        )
                        .to_compile_error()
                        .into())
                    }
                };

                if index >= req_len as u64 {
                    return Err(Error::new(
                        Span::call_site(),
                        format!("invalid middleware::request index {}", index),
                    )
                    .to_compile_error()
                    .into());
                }

                let m_arg_ident = format_ident!("m_arg_{}", index);
                let method_resolve = quote! {
                    let #arg_name: #ttype = #m_arg_ident;
                };
                return Ok(HandlerArgs::Middleware(
                    arg_name,
                    method_resolve,
                    index,
                    *ttype.clone(),
                ));
            }

            if left == "middleware" && right == "response" {
                return Err(
                    Error::new_spanned(left, "handlers args cannot refer to `middleware::response` return values because they are ran post handler")
                        .to_compile_error()
                        .into(),
                );
            }
        }
    }

    Err(Error::new_spanned(attr, "unsupported attribute")
        .to_compile_error()
        .into())
}

#[derive(Default)]
struct MiddlewareCall {
    req: Vec<(u16, TokenStream2)>,
    res: Vec<(u16, TokenStream2)>,
    req_len: usize,
    res_len: usize,
}

fn make_call_middleware(middleware: Option<ReqResArray>) -> MiddlewareCall {
    let mut middleware_req = vec![];
    let mut middleware_res = vec![];
    let (mut req_len, mut res_len) = (0, 0);
    let mut i = 0u16;

    middleware.map(|r| {
        req_len = r.request.map(|rm| {
            for e in &rm {
                let name = e.get_name();
                let m_arg_ident = format_ident!("m_arg_{}", i);
                let mut sorter = 0_u16;
                let m_args: Vec<proc_macro2::TokenStream> =
                    get_req_middleware_arg(e, &mut sorter, rm.len());

                let m_args = if m_args.len() > 1 {
                    quote! {(#(#m_args ,)*)}
                } else if m_args.len() == 1 {
                    quote! {#(#m_args ,)*}
                } else {
                    quote! {()}
                };

                middleware_req.push((sorter, quote! {
                    let #m_arg_ident = match #name::call(&mut args.request, args.container.clone(), #m_args).await {
                        Ok(k) => k,
                        Err(e) => return Ok(e.respond_err()),
                    };
                }));
                i += 1;
            }

            rm.len()
        }).unwrap_or(0);

        res_len = r.response.map(|rm| {
            for e in &rm {
                let name = e.get_name();
                let r_m_arg_ident = format_ident!("res_m_arg_{}", i);
                let mut sorter = 0_u16;
                let m_args: Vec<proc_macro2::TokenStream> =
                    get_res_middleware_arg(e, &mut sorter, rm.len(), res_len);

                let m_args = if m_args.len() > 1 {
                    quote! {(#(#m_args ,)*)}
                } else if m_args.len() == 1 {
                    quote! {#(#m_args ,)*}
                } else {
                    quote! {()}
                };

                middleware_res.push((std::u16::MAX - i - sorter, quote! {
                    let #r_m_arg_ident = match #name::call(&mut rb, args.container.clone(), #m_args).await {
                        Ok(k) => k,
                        Err(e) => return Ok(e.respond_err()),
                    };
                }));
                i += 1;
            }
            rm.len()
        }).unwrap_or(0);
    });
    MiddlewareCall {
        req: middleware_req,
        res: middleware_res,
        req_len,
        res_len,
    }
}

struct JobCall {
    req: Vec<TokenStream2>,
    res: Vec<TokenStream2>,
}

fn make_job_call(jobs: Option<ReqResArray>) -> JobCall {
    let mut jobs_req = vec![];
    let mut jobs_res = vec![];

    jobs.map(|jobs| {
        jobs.request.map(|jr| {
            jr.iter().for_each(|e| {
                let (name, m_args) = match e {
                    Func::Call(ec) => {
                        let name = ec.func.to_token_stream();
                        let m_args: Vec<proc_macro2::TokenStream> = ec
                            .args
                            .iter()
                            .map(|arg| {
                                quote! {#arg}
                            })
                            .collect();

                        let q = if m_args.len() > 1 {
                            quote! {(#(#m_args ,)*)}
                        } else if m_args.len() == 1 {
                            quote! {#(#m_args ,)*}
                        } else {
                            quote! {()}
                        };

                        (name, q)
                    }
                    Func::Path(path) => (path.to_token_stream(), quote! {()}),
                };

                jobs_req.push(quote! {
                    darpi::job::assert_request_job(#name);
                    match #name::call(&r, args.container.clone(), #m_args).await.into() {
                        darpi::job::Job::CpuBound(function) => {
                            let res = darpi::spawn(function).await;
                            if let Err(e) = res {
                                log::warn!("could not queue CpuBound job err: {}", e);
                            }
                        }
                        darpi::job::Job::IOBlocking(function) => {
                            let res = darpi::spawn(function).await;
                            if let Err(e) = res {
                                log::warn!("could not queue IOBlocking job err: {}", e);
                            }
                        }
                        darpi::job::Job::Future(fut) => {
                            let res = darpi::spawn(fut).await;
                            if let Err(e) = res {
                                log::warn!("could not queue Future job err: {}", e);
                            }
                        }
                    };
                });
            });
        });

        jobs.response.map(|ref mut jr| {
            jr.iter_mut().for_each(|e| {
                let (name, m_args) = match e {
                    Func::Call(ec) => {
                        let name = ec.func.to_token_stream();
                        let m_args: Vec<proc_macro2::TokenStream> = ec
                            .args
                            .iter()
                            .map(|arg| {
                                quote! {#arg}
                            })
                            .collect();

                        let q = if m_args.len() > 1 {
                            quote! {(#(#m_args ,)*)}
                        } else if m_args.len() == 1 {
                            quote! {#(#m_args ,)*}
                        } else {
                            quote! {()}
                        };
                        (name, q)
                    }
                    Func::Path(p) => (p.to_token_stream(), quote! {()}),
                };

                jobs_res.push(quote! {
                    darpi::job::assert_response_job(#name);
                    match #name::call(&rb, args.container.clone(), #m_args).await.into() {
                        darpi::job::Job::CpuBound(function) => {
                            let res = darpi::spawn(function).await;
                            if let Err(e) = res {
                                log::warn!("could not queue CpuBound job err: {}", e);
                            }
                        }
                        darpi::job::Job::IOBlocking(function) => {
                            let res = darpi::spawn(function).await;
                            if let Err(e) = res {
                                log::warn!("could not queue IOBlocking job err: {}", e);
                            }
                        }
                        darpi::job::Job::Future(fut) => {
                            let res = darpi::spawn(fut).await;
                            if let Err(e) = res {
                                log::warn!("could not queue Future job err: {}", e);
                            }
                        }
                    };
                });
            });
        });
    });
    JobCall {
        req: jobs_req,
        res: jobs_res,
    }
}

#[derive(Debug)]
pub struct Config {
    pub(crate) container: Option<syn::Path>,
    pub(crate) jobs: Option<ReqResArray>,
    pub(crate) middleware: Option<ReqResArray>,
}

impl Parse for Config {
    fn parse(input: ParseStream) -> SynResult<Self> {
        let content;
        let _ = braced!(content in input);

        let mut container: Option<syn::Path> = None;
        let mut jobs: Option<ReqResArray> = None;
        let mut middleware: Option<ReqResArray> = None;

        while !content.is_empty() {
            if content.peek(token::Comma) {
                let _: token::Comma = content.parse()?;
            }

            let key = content.fork().parse::<Ident>()?;

            if key == "container" {
                let _: Ident = content.parse()?;
                let _: token::Colon = content.parse()?;
                let c: syn::Path = content.parse()?;
                container = Some(c);
                continue;
            }
            if key == "jobs" {
                let j: ReqResArray = content.parse()?;
                jobs = Some(j);
                continue;
            }
            if key == "middleware" {
                let m: ReqResArray = content.parse()?;
                middleware = Some(m);
                continue;
            }

            return Err(Error::new_spanned(
                key.clone(),
                format!(
                    "unknown key: `{}`. Only `route`, `handler` and `method` are allowed",
                    key
                ),
            ));
        }

        return Ok(Config {
            container,
            jobs,
            middleware,
        });
    }
}
