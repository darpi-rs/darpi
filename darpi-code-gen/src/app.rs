use proc_macro::TokenStream;
use proc_macro2::Span;
use quote::ToTokens;
use quote::{format_ident, quote};
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

    let handlers = config.handlers;

    let (module_def, module_let, module_self) = config.container.map_or(
        {
            (
                quote! {module: std::sync::Arc<darpi::EmptyContainer>,},
                quote! {let module = std::sync::Arc::new(darpi::make_empty_container());},
                quote! {module: module,},
            )
        },
        |mp| {
            let patj = mp.ttype;
            let make_container_func = mp.factory;

            (
                quote! {module: std::sync::Arc<#patj>,},
                quote! {let module = std::sync::Arc::new(#make_container_func);},
                quote! {module: module,},
            )
        },
    );

    let (middleware_req, middleware_res) =
        config.middleware.map_or(Default::default(), |middleware| {
            let mut middleware_req = vec![];
            let mut middleware_res = vec![];
            let mut i = 0u16;

            middleware.request.map(|rm| {
               rm.iter().for_each(|e| {
                    let m_arg_ident = format_ident!("m_arg_{}", i);

                    let (name, m_args) = match e {
                        Func::Call(expr_call) => {
                            let m_args: Vec<proc_macro2::TokenStream> = expr_call.args.iter().map(|arg| {
                                if let SynExpr::Call(expr_call) = arg {
                                    if expr_call.func.to_token_stream().to_string() == "request" {
                                        let index: u16 = expr_call.args.first().unwrap().to_token_stream().to_string().parse().unwrap();
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


                    middleware_req.push(quote! {
                    let #m_arg_ident = match #name::call(&mut r, inner_module.clone(), #m_args).await {
                        Ok(k) => k,
                        Err(e) => return Ok(e.respond_err()),
                    };
                });
                    i += 1;
                });
            });

            let mut i = 0u16;

            middleware.response.map(|ref mut rm| {
                rm.iter_mut().for_each(|e| {
                    let r_m_arg_ident = format_ident!("res_m_arg_{}", i);
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
                                        let r_m_arg_ident = format_ident!("res_m_arg_{}", index);
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
                                                let r_m_arg_ident = format_ident!("res_m_arg_{}", index);
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

                    middleware_res.push(quote! {
                    let #r_m_arg_ident = match #name::call(&mut rb, inner_module.clone(), #m_args).await {
                        Ok(k) => k,
                        Err(e) => return Ok(e.respond_err()),
                    };
                });
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
                    match #name::call(&r, inner_module.clone(), #m_args).await.into() {
                        darpi::job::Job::CpuBound(function) => {
                                let res = darpi::spawn(function);
                                if let Err(e) = res {
                                    log::warn!("could not queue CpuBound job err: {}", e);
                                }
                            }
                            darpi::job::Job::IOBlocking(function) => {
                                let res = darpi::spawn(function);
                                if let Err(e) = res {
                                    log::warn!("could not queue IOBlocking job err: {}", e);
                                }
                            }
                            darpi::job::Job::Future(fut) => {
                                let res = darpi::spawn(fut);
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
                                let res = darpi::spawn(function);
                                if let Err(e) = res {
                                    log::warn!("could not queue CpuBound job err: {}", e);
                                }
                            }
                            darpi::job::Job::IOBlocking(function) => {
                                let res = darpi::spawn(function);
                                if let Err(e) = res {
                                    log::warn!("could not queue IOBlocking job err: {}", e);
                                }
                            }
                            darpi::job::Job::Future(fut) => {
                                let res = darpi::spawn(fut);
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

    let mut route_defs = vec![];
    let mut route_strs = vec![];
    let mut route_match = vec![];

    for (i, h) in handlers.iter().enumerate() {
        let id = format_ident!("route{}", i);
        route_strs.push(h.route.clone());
        let ha = h.handler.clone();

        route_match.push(quote! {
            #i => {
                 if #id::is_match(method.as_str()) {
                    let args_vec = rm.get_args().to_vec();
                    let args = darpi::Args{
                        request: r,
                        container: inner_module.clone(),
                        route_args: #id::get_tuple_args(&route_str, &args_vec),
                    };
                    let mut rb = Handler::call(#ha, args).await.unwrap();
                    #(#middleware_res )*
                    #(#jobs_res )*
                    return Ok::<_, std::convert::Infallible>(rb);
                }
            }
        });

        let r = make_route_lit(
            &id,
            &h.method.path.segments.last().unwrap().ident.to_string(),
            h.route.to_token_stream(),
        )?;
        route_defs.push(r);
    }

    let app = quote! {
        static __ONCE_INTERNAL__: std::sync::Once = std::sync::Once::new();
        #(#route_defs )*

         pub struct AppImpl {
            #module_def
            router: std::sync::Arc<darpi::gonzales::Router>,
            address: std::net::SocketAddr,
            rx: tokio::sync::oneshot::Receiver<()>,
            tx: Option<tokio::sync::oneshot::Sender<()>>,
            start_rx: Option<tokio::sync::oneshot::Receiver<()>>,
            start_tx: Option<tokio::sync::oneshot::Sender<()>>
        }

        impl AppImpl {
            fn new(address: &str) -> Self {
                let (tx, rx) = tokio::sync::oneshot::channel::<()>();
                let address: std::net::SocketAddr = address
                    .parse()
                    .expect(&format!("invalid server address: `{}`", address));

                let routes_vec = vec![#(#route_strs ,)*];
                let router = std::sync::Arc::new(darpi::gonzales::RouterBuilder::new().build(routes_vec));

                #module_let
                Self {
                    #module_self
                    router: router,
                    address: address,
                    rx: rx,
                    tx: Some(tx),
                    start_rx: None,
                    start_tx: None,
                }
            }
        }

        #[darpi::async_trait]
        impl darpi::App for AppImpl {
            fn startup_notify(&mut self) -> Option<tokio::sync::oneshot::Receiver<()>> {
                if let Some(_) = self.start_tx {
                    return None;
                }
                let (tx, rx) = tokio::sync::oneshot::channel::<()>();
                self.start_tx = Some(tx);
                Some(rx)
            }

            fn shutdown_signal(&mut self) -> Option<tokio::sync::oneshot::Sender<()>> {
                self.tx.take()
            }

             async fn run(self) -> Result<(), darpi::Error> {
                let address = self.address;
                let module = self.module.clone();
                let router = self.router.clone();
                let start_tx = self.start_tx;
                let rx = self.rx;

                let default_hook = std::panic::take_hook();
                std::panic::set_hook(Box::new(move |panic| {
                    darpi::log::error!("panic reason:  `{}`", panic);
                    default_hook(panic);
                }));

                __ONCE_INTERNAL__.call_once(|| {
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
                });

                let make_svc = darpi::service::make_service_fn(move |_conn| {
                    let inner_module = std::sync::Arc::clone(&module);
                    let inner_router = std::sync::Arc::clone(&router);

                    async move {
                        Ok::<_, std::convert::Infallible>(darpi::service::service_fn(move |mut r: darpi::Request<darpi::Body>| {
                            use darpi::futures::FutureExt;
                            use darpi::response::ResponderError;
                            #[allow(unused_imports)]
                            use darpi::RequestMiddleware;
                            #[allow(unused_imports)]
                            use darpi::ResponseMiddleware;
                            use darpi::{RequestJobFactory, ResponseJobFactory};
                            use darpi::Handler;
                            use darpi::Route;
                            let inner_module = std::sync::Arc::clone(&inner_module);
                            let inner_router = std::sync::Arc::clone(&inner_router);

                            async move {
                                let route_str = r.uri().path().to_string();
                                let method = r.method().clone();

                                #(#middleware_req )*
                                #(#jobs_req )*

                                let route_m = inner_router.route(&route_str);

                                if let Some(rm) = route_m {
                                    match rm.get_index() {
                                        #(#route_match ,)*
                                        _ => {}
                                    }
                                }

                                return  async {
                                     Ok::<_, std::convert::Infallible>(darpi::Response::builder()
                                        .status(darpi::StatusCode::NOT_FOUND)
                                        .body(darpi::Body::empty())
                                        .unwrap())
                                }.await;
                            }
                        }))
                    }
                });

                let server = darpi::Server::bind(&address).serve(make_svc);
                let graceful = server.with_graceful_shutdown(async { rx.await.ok(); });
                if let Some(start) = start_tx {
                    let _ = start.send(());
                }
                graceful.await
             }
        }
    };

    let tokens = quote! {
        {
            #app
            AppImpl::new(#address_value)
        }
    };

    //panic!("{}", tokens.to_string());
    Ok(tokens.into())
}

fn make_route_lit(
    struct_ident: &Ident,
    method_type: &str,
    r: proc_macro2::TokenStream,
) -> Result<proc_macro2::TokenStream, syn::Error> {
    let r_str = r.to_token_stream().to_string();
    let mut parts = vec![];
    let mut args = vec![];
    let mut args_i = 0;

    for (i, part) in r_str.split('/').enumerate() {
        let part = part.trim_end_matches('"');
        let starts = part.starts_with('{');
        let ends = part.ends_with('}');
        if starts ^ ends {
            return Err(Error::new_spanned(
                r,
                "route arguments must start with `{` and end with `}`",
            ));
        }

        if starts && ends {
            args.push((args_i, part.to_string()));
            args_i += 1;
        } else {
            parts.push((i, part.to_string()));
        }
    }

    let method_str_lit = LitStr::new(method_type, Span::call_site());
    let mut prop_values = HashMap::new();

    for (i, prop) in args.iter() {
        let prop_name = format_ident!("arg{}", i);
        prop_values.insert((prop_name.clone(), i), prop);
    }

    let mut tuple_type = vec![];
    let mut get_args_lines = vec![];
    for ((_, index), sorter) in prop_values {
        let i = syn::Index::from(*index);
        get_args_lines.push((quote! {route_str[r[#i].0..r[#i].1].to_string()}, sorter));
        tuple_type.push(quote! {String});
    }

    get_args_lines.sort_by(|a, b| a.1.cmp(b.1));

    let sorted_get_args_lines: Vec<proc_macro2::TokenStream> =
        get_args_lines.iter().map(|a| a.0.clone()).collect();

    Ok(quote! {
        struct #struct_ident;

        impl darpi::Route<(#(#tuple_type ,)*)> for #struct_ident {
            #[inline(always)]
            fn is_match(method: &str) -> bool {
                #method_str_lit == method
            }

            #[inline(always)]
            fn get_tuple_args(route_str: &str, r: &Vec<(usize, usize)>) -> (#(#tuple_type ,)*) {
                (#(#sorted_get_args_lines ,)*)
            }
        }
    })
}

#[derive(Debug)]
pub(crate) enum Address {
    Lit(LitStr),
    Ident(Ident),
}

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

        let methods = vec![
            http::Method::GET.to_string(),
            http::Method::POST.to_string(),
            http::Method::PUT.to_string(),
            http::Method::DELETE.to_string(),
            http::Method::HEAD.to_string(),
            http::Method::OPTIONS.to_string(),
            http::Method::CONNECT.to_string(),
            http::Method::PATCH.to_string(),
            http::Method::TRACE.to_string(),
        ];
        let method_str = method.to_token_stream().to_string();
        if !methods.contains(&method_str) {
            return Err(SynError::new_spanned(
                method,
                format!(
                    "Invalid method `{}`. Available methods {:#?}",
                    method_str, methods
                ),
            ));
        }

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
