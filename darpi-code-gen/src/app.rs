//use crate::handler::{HAS_NO_PATH_ARGS_PREFIX, HAS_PATH_ARGS_PREFIX, NO_BODY_PREFIX};
use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::ToTokens;
use quote::{format_ident, quote};
use std::cmp::Ordering;
use std::collections::HashMap;
use syn::parse::{Error as SynError, Parse, ParseStream, Result as SynResult};
use syn::parse_quote::ParseQuote;

use syn::{
    braced, bracketed, punctuated::Punctuated, token, Error, Expr as SynExpr, Expr, ExprCall,
    ExprLit, ExprPath, Ident, LitStr,
};

pub(crate) fn make_app(config: Config) -> Result<TokenStream, SynError> {
    let address_value = {
        let av = match &config.address {
            Address::Ident(id) => id.to_token_stream(),
            Address::Lit(lit) => lit.to_token_stream(),
        };
        let q = quote! {&#av};
        q.to_token_stream()
    };

    if config.handlers.is_empty() {
        return Err(Error::new(Span::call_site(), "no handlers registered"));
    }

    let handler_len = config.handlers.len();
    let handlers = config.handlers;

    let HandlerTokens {
        routes,
        route_arg_assert,
        route_arg_assert_def,
        routes_match,
        is,
        body_assert,
        body_assert_def,
    } = make_handlers(handlers)?;

    let route_possibilities = quote! {
        use std::convert::TryFrom;
        #[allow(non_camel_case_types, missing_docs)]
        pub enum RoutePossibilities {
            #(#routes ,)*
        }

        impl RoutePossibilities {
            pub fn get_route<'a>(&self, route: &'a str, method: &darpi::Method) -> Option<(darpi::ReqRoute<'a>, std::collections::HashMap<&'a str, &'a str>)> {
                return match self {
                    #(#is ,)*
                };
            }
        }
    };

    let (module_def, module_let, module_self) = config.container.map_or(Default::default(), |mp| {
        let patj = mp.ttype;
        let make_container_func = mp.factory;

        (
            quote! {module: std::sync::Arc<#patj>,},
            quote! {let module = std::sync::Arc::new(#make_container_func);},
            quote! {module: module,},
        )
    });

    let (mut middleware_req, mut middleware_res) =
        config.middleware.map_or(Default::default(), |middleware| {
            let mut middleware_req = vec![];
            let mut middleware_res = vec![];
            let mut i = 0u16;

            middleware.request.map(|rm| {
               rm.iter().for_each(|e| {
                    let m_arg_ident = format_ident!("m_arg_{}", i);
                    let mut sorter = 0_u16;

                    let (name, m_args) = match e {
                        Func::Call(expr_call) => {
                            let m_args: Vec<proc_macro2::TokenStream> = expr_call.args.iter().map(|arg| {
                                if let SynExpr::Call(expr_call) = arg {
                                    if expr_call.func.to_token_stream().to_string() == "request" {
                                        let index: u16 = expr_call.args.first().unwrap().to_token_stream().to_string().parse().unwrap();
                                        sorter += index;
                                        let i_ident = format_ident!("m_arg_{}", index);
                                        return quote!{#i_ident.clone()};
                                    }
                                }
                                quote! {#arg}
                            }).collect();

                            let q = if m_args.len() > 1 {
                                quote! {(#(#m_args ,)*)}
                            } else if m_args.len() == 1 {
                                quote! {#(#m_args ,)*}
                            } else {
                                quote! {()}
                            };

                            (expr_call.func.to_token_stream(), q)
                        },
                        Func::Path(expr_path) => {
                            (expr_path.to_token_stream(), quote! {()})
                        }
                    };


                    middleware_req.push((sorter, quote! {
                    let #m_arg_ident = match #name::call(&mut parts, inner_module.clone(), &mut body, #m_args).await {
                        Ok(k) => k,
                        Err(e) => return Ok(e.respond_err()),
                    };
                }));
                    i += 1;
                });
            });

            middleware.response.map(|ref mut rm| {
                rm.iter_mut().for_each(|e| {
                    let r_m_arg_ident = format_ident!("res_m_arg_{}", i);
                    let mut sorter = 0_u16;

                    let (name, m_args) = match e {
                        Func::Call(expr_call) => {
                            let m_args: Vec<proc_macro2::TokenStream> = expr_call.args.iter_mut().map(|arg| {
                                if let SynExpr::Call(expr_call) = arg {
                                    if expr_call.func.to_token_stream().to_string() == "request" {
                                        let index: u16 = expr_call.args.first().unwrap().to_token_stream().to_string().parse().unwrap();
                                        let i_ident = format_ident!("m_arg_{}", index);
                                        return quote!{#i_ident.clone()};
                                    }
                                    if expr_call.func.to_token_stream().to_string() == "response" {
                                        let index: u16 = expr_call.args.first().unwrap().to_token_stream().to_string().parse().unwrap();
                                        sorter += index;
                                        return quote!{#r_m_arg_ident.clone()};
                                    }
                                }
                                if  let SynExpr::Tuple(tuple) = arg.clone() {
                                    let tuple_expr: Vec<proc_macro2::TokenStream> = tuple.elems.iter().map(|tuple_arg| {
                                        if let SynExpr::Call(expr_call) = tuple_arg {
                                            if expr_call.func.to_token_stream().to_string() == "request" {
                                                let index: u16 = expr_call.args.first().unwrap().to_token_stream().to_string().parse().unwrap();
                                                let i_ident = format_ident!("m_arg_{}", index);
                                                return quote!{#i_ident.clone()};
                                            }
                                            if expr_call.func.to_token_stream().to_string() == "response" {
                                                let index: u16 = expr_call.args.first().unwrap().to_token_stream().to_string().parse().unwrap();
                                                sorter += index;
                                                return quote!{#r_m_arg_ident.clone()};
                                            }
                                        }
                                        quote! {#tuple_arg}
                                    }).collect();
                                    return quote! {( #(#tuple_expr ,)* )};
                                }
                                quote! {#arg}
                            }).collect();

                            let q = if m_args.len() > 1 {
                                quote! {(#(#m_args ,)*)}
                            } else if m_args.len() == 1 {
                                quote! {#(#m_args ,)*}
                            } else {
                                quote! {()}
                            };

                            (expr_call.func.to_token_stream(), q)

                        },
                        Func::Path(expr_path) => {
                            (expr_path.to_token_stream(), quote! {()})
                        }
                    };

                    middleware_res.push((std::u16::MAX - i - sorter, quote! {
                    let #r_m_arg_ident = match #name::call(&mut rb, inner_module.clone(), #m_args).await {
                        Ok(k) => k,
                        Err(e) => return Ok(e.respond_err()),
                    };
                }));
                    i += 1;
                });
            });

            (
                middleware_req,
                middleware_res,
            )
        });

    let (jobs_req, jobs_res) = config.jobs.map_or(Default::default(), |jobs| {
        let mut jobs_req = vec![];
        let mut jobs_res = vec![];

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
                    match #name::call(&parts, inner_module.clone(), &body, #m_args).await.into() {
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
                    match #name::call(&rb, inner_module.clone(), #m_args).await.into() {
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

        (jobs_req, jobs_res)
    });

    middleware_req.sort_by(|a, b| a.0.cmp(&b.0));
    middleware_res.sort_by(|a, b| a.0.cmp(&b.0));

    let middleware_req: Vec<proc_macro2::TokenStream> =
        middleware_req.into_iter().map(|e| e.1).collect();
    let middleware_res: Vec<proc_macro2::TokenStream> =
        middleware_res.into_iter().map(|e| e.1).collect();

    let app = quote! {
        #(#body_assert_def )*
        #(#route_arg_assert_def )*

         pub struct App {
            #module_def
            handlers: std::sync::Arc<[RoutePossibilities; #handler_len]>,
            address: std::net::SocketAddr,
        }

        impl App {
            pub fn new(address: &str) -> Self {
                #(#body_assert;)*
                #(#route_arg_assert;)*
                let address: std::net::SocketAddr = address
                    .parse()
                    .expect(&format!("invalid server address: `{}`", address));

                #module_let
                Self {
                    #module_self
                    handlers: std::sync::Arc::new([#(RoutePossibilities::#routes ,)*]),
                    address: address,
                }
            }

             pub async fn run(self) -> Result<(), darpi::Error> {
                let address = self.address;
                let module = self.module.clone();
                let handlers = self.handlers.clone();

                std::panic::set_hook(Box::new(|panic| {
                    darpi::log::warn!("panic reason:  `{}`", panic);
                }));

                darpi::rayon::ThreadPoolBuilder::new()
                    .panic_handler(|panic| {
                        let msg = match panic.downcast_ref::<&'static str>() {
                            Some(s) => *s,
                            None => match panic.downcast_ref::<String>() {
                                Some(s) => &s[..],
                                None => "Unknown",
                            },
                        };
                        darpi::log::warn!("panic reason:  `{}`", msg);
                    })
                    .build_global().unwrap();

                let make_svc = darpi::service::make_service_fn(move |_conn| {
                    let inner_module = std::sync::Arc::clone(&module);
                    let inner_handlers = std::sync::Arc::clone(&handlers);

                    async move {
                        Ok::<_, std::convert::Infallible>(darpi::service::service_fn(move |r: darpi::Request<darpi::Body>| {
                            use darpi::futures::FutureExt;
                            use darpi::response::ResponderError;
                            #[allow(unused_imports)]
                            use darpi::RequestMiddleware;
                            #[allow(unused_imports)]
                            use darpi::ResponseMiddleware;
                            use darpi::{RequestJobFactory, ResponseJobFactory};
                            use darpi::Handler;
                            let inner_module = std::sync::Arc::clone(&inner_module);
                            let inner_handlers = std::sync::Arc::clone(&inner_handlers);

                            async move {
                                let route = r.uri().path().to_string();
                                let method = r.method().clone();

                                let (mut parts, mut body) = r.into_parts();

                                #(#middleware_req )*
                                #(#jobs_req )*

                                let mut handler = None;
                                for rp in inner_handlers.iter() {
                                    if let Some(rr) = rp.get_route(&route, &method) {
                                        handler = Some((rp, rr));
                                        break;
                                    }
                                }

                                let handler = match handler {
                                    Some(s) => s,
                                    None => return  async {
                                         Ok::<_, std::convert::Infallible>(darpi::Response::builder()
                                                .status(darpi::StatusCode::NOT_FOUND)
                                                .body(darpi::Body::empty())
                                                .unwrap())
                                    }.await,
                                };

                                let mut rb = match handler.0 {
                                    #(#routes_match ,)*
                                };

                                if let Ok(mut rb) = rb.as_mut() {
                                    #(#middleware_res )*
                                    #(#jobs_res )*
                                }

                                rb
                            }
                        }))
                    }
                });

                let server = darpi::Server::bind(&address).serve(make_svc);
                server.await
             }
        }
    };

    let tokens = quote! {
        {
            #route_possibilities
            #app
            App::new(#address_value)
        }
    };
    //panic!("{}", tokens.to_string());
    Ok(tokens.into())
}

struct HandlerTokens {
    routes: Vec<proc_macro2::TokenStream>,
    route_arg_assert: Vec<proc_macro2::TokenStream>,
    route_arg_assert_def: Vec<proc_macro2::TokenStream>,
    routes_match: Vec<proc_macro2::TokenStream>,
    is: Vec<proc_macro2::TokenStream>,
    body_assert: Vec<proc_macro2::TokenStream>,
    body_assert_def: Vec<proc_macro2::TokenStream>,
}

fn make_handlers(handlers: Punctuated<Handler, token::Comma>) -> Result<HandlerTokens, SynError> {
    let mut is = vec![];
    let mut routes = vec![];
    let mut routes_match = vec![];
    let body_assert = vec![];
    let body_assert_def = vec![];
    let route_arg_assert = vec![];
    let route_arg_assert_def = vec![];

    for el in handlers.iter() {
        let handler = el
            .handler
            .path
            .segments
            .last()
            .expect("cannot get handler segment")
            .ident
            .clone();

        let method = el.method.path.segments.to_token_stream();
        let route = el.route.clone();
        let variant_name = format!("{}{}", handler.clone(), method.clone());
        let variant_name: String = variant_name
            .chars()
            .map(|ch| {
                if ch.is_alphanumeric() {
                    return ch;
                }
                '_'
            })
            .collect();

        let variant_name = format_ident!("{}", variant_name);
        let variant_value = el
            .handler
            .path
            .get_ident()
            .expect("cannot get handler path ident");

        let method_name = el.method.path.segments.last().unwrap();
        // let mut f_name = format_ident!("assert_has_no_path_args_{}", variant_value);
        // let mut t_name = format_ident!("{}_{}", HAS_NO_PATH_ARGS_PREFIX, variant_value);

        if route.clone().to_token_stream().to_string().contains('{') {
            // f_name = format_ident!("assert_has_path_args_{}", variant_value);
            // t_name = format_ident!("{}_{}", HAS_PATH_ARGS_PREFIX, variant_value);
        }

        //todo fix use the handler path
        //route_arg_assert_def.push(quote! {fn #f_name<T>() where T: #t_name {}});
        // route_arg_assert.push(quote! {
        //     #f_name::<#variant_value>();
        // });

        if method_name.ident == "GET" {
            // let f_name = format_ident!("assert_no_body_{}", variant_value);
            // let t_name = format_ident!("{}_{}", NO_BODY_PREFIX, variant_value);
            // body_assert_def.push(quote! {fn #f_name<T>() where T: #t_name {}});
            // body_assert.push(quote! {
            //     #f_name::<#variant_value>();
            // });
        }

        is.push(quote! {
            RoutePossibilities::#variant_name => {
                let req_route = darpi::ReqRoute::try_from(route).unwrap();
                let def_route = darpi::Route::try_from(#route).unwrap();
                if def_route == req_route && method == #method.as_str() {
                    let args = req_route.extract_args(&def_route).unwrap();
                    return Some((req_route, args));
                }
                None
            }
        });

        let route_str = route.to_token_stream().to_string();
        routes.push((
            quote! {
                #variant_name
            },
            route_str,
        ));

        routes_match.push(quote! {
            RoutePossibilities::#variant_name => {
                let args = darpi::Args{
                    request_parts: &mut parts,
                    container: inner_module.clone(),
                    body: body,
                    route_args: handler.1.1,
                };
                Handler::call(&#variant_value, args).await
            }
        });
    }

    routes.sort_by(|left, right| {
        let left_matches: Vec<usize> = left.1.match_indices('{').map(|t| t.0).collect();

        if left_matches.is_empty() {
            return Ordering::Less;
        }

        let left_count = left_matches.iter().fold(0, |acc, a| acc + a);
        let right_matches: Vec<usize> = right.1.match_indices('{').map(|t| t.0).collect();

        if right_matches.is_empty() {
            return Ordering::Greater;
        }

        let right_count = right_matches.iter().fold(0, |acc, a| acc + a);

        if left_matches.len() + left_count > right_matches.len() + right_count {
            return Ordering::Less;
        }

        Ordering::Greater
    });

    let routes: Vec<proc_macro2::TokenStream> = routes.into_iter().map(|(ts, _)| ts).collect();

    Ok(HandlerTokens {
        routes,
        route_arg_assert,
        route_arg_assert_def,
        routes_match,
        is,
        body_assert,
        body_assert_def,
    })
}

#[derive(Debug)]
pub(crate) enum Address {
    Lit(LitStr),
    Ident(Ident),
}

// impl ToTokens for Address {
//     fn to_tokens(&self, tokens: &mut TokenStream) {
//         match self {
//             Self::Lit(lit) => lit.to_token_stream().to_tokens(tokens),
//             Self::Ident(id) => id.to_tokens(tokens),
//         }
//     }
// }

impl Parse for Address {
    fn parse(input: ParseStream) -> SynResult<Self> {
        if input.peek(LitStr) {
            let lit_str: LitStr = input.parse()?;
            return Ok(Address::Lit(lit_str));
        }
        let ident: Ident = input.parse()?;
        Ok(Address::Ident(ident))
    }
}

#[derive(Debug)]
pub(crate) struct Container {
    pub factory: ExprCall,
    pub ttype: syn::Path,
}

impl Parse for Container {
    fn parse(input: ParseStream) -> SynResult<Self> {
        let name: Ident = input.parse()?;
        let _: token::Colon = input.parse()?;

        let mut factory: Option<ExprCall> = None;
        let mut ttype: Option<syn::Path> = None;

        let content;
        let _ = braced!(content in input);

        while !content.is_empty() {
            if content.peek(token::Comma) {
                let _: token::Comma = content.parse()?;
            }

            let key: Ident = if content.peek(token::Type) {
                let _: token::Type = content.parse()?;
                format_ident!("type")
            } else {
                content.parse()?
            };
            let _: token::Colon = content.parse()?;

            if key == "factory" {
                let f: ExprCall = content.parse()?;
                factory = Some(f);
                continue;
            }
            if key == "type" {
                let t: syn::Path = content.parse()?;
                ttype = Some(t);
                continue;
            }

            return Err(Error::new_spanned(
                key.clone(),
                format!(
                    "unknown key: `{}`. Only `factory` and `type` are allowed",
                    key
                ),
            ));
        }

        let factory = match factory {
            Some(r) => r,
            None => return Err(SynError::new_spanned(name, "missing `factory`")),
        };

        let ttype = match ttype {
            Some(r) => r,
            None => return Err(Error::new_spanned(name, "missing `type`")),
        };

        Ok(Container { factory, ttype })
    }
}

#[derive(Debug)]
pub(crate) enum Func {
    Call(ExprCall),
    Path(ExprPath),
}

impl Func {
    pub fn get_name(&self) -> proc_macro2::TokenStream {
        match self {
            Self::Call(ec) => ec.func.to_token_stream(),
            Self::Path(ep) => ep.to_token_stream(),
        }
    }
    pub fn get_args(&self) -> Punctuated<Expr, token::Comma> {
        match self {
            Self::Call(ec) => ec.args.clone(),
            Self::Path(_) => Default::default(),
        }
    }
}

impl ToTokens for Func {
    fn to_tokens(&self, tokens: &mut proc_macro2::TokenStream) {
        match self {
            Self::Call(ec) => ec.to_tokens(tokens),
            Self::Path(ep) => ep.to_tokens(tokens),
        }
    }
}

impl Parse for Func {
    fn parse(input: ParseStream) -> SynResult<Self> {
        if input.fork().parse::<ExprCall>().is_ok() {
            let expr_call: ExprCall = input.parse()?;
            return Ok(Func::Call(expr_call));
        }

        let expr_path: ExprPath = input.parse()?;
        return Ok(Func::Path(expr_path));
    }
}

#[derive(Debug)]
pub(crate) struct ReqResArray {
    pub request: Option<Punctuated<Func, token::Comma>>,
    pub response: Option<Punctuated<Func, token::Comma>>,
}

impl Parse for ReqResArray {
    fn parse(input: ParseStream) -> SynResult<Self> {
        let _: Ident = input.parse()?;
        let _: token::Colon = input.parse()?;

        let content;
        let _ = braced!(content in input);

        let mut request: Option<Punctuated<Func, token::Comma>> = None;
        let mut response: Option<Punctuated<Func, token::Comma>> = None;

        while !content.is_empty() {
            if content.peek(token::Comma) {
                let _: token::Comma = content.parse()?;
            }

            if !content.peek(Ident) {
                break;
            }
            let key: Ident = content.parse()?;
            let _: token::Colon = content.parse()?;

            let brc;
            let _ = bracketed!(brc in content);

            if key == "request" {
                let r: Punctuated<Func, token::Comma> = Punctuated::parse(&brc)?;
                request = Some(r);
                continue;
            }
            if key == "response" {
                let r: Punctuated<Func, token::Comma> = Punctuated::parse(&brc)?;
                response = Some(r);
                continue;
            }

            return Err(Error::new_spanned(
                key.clone(),
                format!(
                    "unknown key: `{}`. Only `request` and `response` are allowed",
                    key
                ),
            ));
        }

        return Ok(ReqResArray { request, response });
    }
}

#[derive(Debug)]
pub struct Config {
    pub(crate) address: Address,
    pub(crate) container: Option<Container>,
    pub(crate) jobs: Option<ReqResArray>,
    pub(crate) middleware: Option<ReqResArray>,
    pub(crate) handlers: Punctuated<Handler, token::Comma>,
}

impl Parse for Config {
    fn parse(input: ParseStream) -> SynResult<Self> {
        let content;
        let _ = braced!(content in input);

        let mut address: Option<Address> = None;
        let mut container: Option<Container> = None;
        let mut jobs: Option<ReqResArray> = None;
        let mut middleware: Option<ReqResArray> = None;
        let mut handlers: Option<Punctuated<Handler, token::Comma>> = None;

        while !content.is_empty() {
            if content.peek(token::Comma) {
                let _: token::Comma = content.parse()?;
            }

            let key = content.fork().parse::<Ident>()?;

            if key == "address" {
                let _: Ident = content.parse()?;
                let _: token::Colon = content.parse()?;
                let a: Address = content.parse()?;
                address = Some(a);
                continue;
            }
            if key == "container" {
                let c: Container = content.parse()?;
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

            if key == "handlers" {
                let _: Ident = content.parse()?;
                let _: token::Colon = content.parse()?;
                let br;
                let _ = bracketed!(br in content);
                let h: Punctuated<Handler, token::Comma> = Punctuated::parse(&br)?;

                let mut handler_validation = HashMap::new();

                for h in h.iter() {
                    let key = format!(
                        "{}{}",
                        h.route.lit.to_token_stream(),
                        h.method.path.to_token_stream()
                    );

                    if handler_validation.get(&key).is_some() {
                        return Err(SynError::new(
                            h.brace.span,
                            "identical handler already defined",
                        ));
                    }
                    handler_validation.insert(key, ());
                }

                handlers = Some(h);
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

        let address = match address {
            Some(r) => r,
            None => return Err(SynError::new(Span::call_site(), "missing `address`")),
        };

        let handlers = match handlers {
            Some(r) => r,
            None => return Err(SynError::new(Span::call_site(), "missing `handlers`")),
        };

        return Ok(Config {
            address,
            container,
            jobs,
            middleware,
            handlers,
        });
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Handler {
    brace: token::Brace,
    route: ExprLit,
    method: ExprPath,
    handler: ExprPath,
}

impl Parse for Handler {
    fn parse(input: ParseStream) -> SynResult<Self> {
        let content;
        let brace = braced!(content in input);
        let mut route: Option<ExprLit> = None;
        let mut method: Option<ExprPath> = None;
        let mut handler: Option<ExprPath> = None;

        while !content.is_empty() {
            if content.peek(token::Comma) {
                let _: token::Comma = content.parse()?;
            }
            let key: Ident = content.parse()?;
            let _: token::Colon = content.parse()?;

            if key == "route" {
                let r: ExprLit = content.parse()?;
                route = Some(r);
                continue;
            }
            if key == "method" {
                let m: ExprPath = content.parse()?;
                method = Some(m);
                continue;
            }
            if key == "handler" {
                let h: ExprPath = content.parse()?;
                handler = Some(h);
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

        let route = match route {
            Some(r) => r,
            None => return Err(SynError::new(brace.span, "missing `route`")),
        };

        let method = match method {
            Some(r) => r,
            None => return Err(SynError::new(brace.span, "missing `method`")),
        };

        let handler = match handler {
            Some(r) => r,
            None => return Err(SynError::new(brace.span, "missing `handler`")),
        };

        return Ok(Handler {
            brace,
            route,
            method,
            handler,
        });
    }
}
