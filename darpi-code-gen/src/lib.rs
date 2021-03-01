#![forbid(unsafe_code)]

mod app;
mod handler;
mod job;
mod logger;
mod middleware;
mod request;

use proc_macro::TokenStream;
use proc_macro2::Ident;
use quote::{format_ident, quote, ToTokens};
use syn::{parse, parse_macro_input, Error, ExprLit, ItemStruct, Pat, PatType, Type};

#[proc_macro_derive(Path)]
pub fn from_path(input: TokenStream) -> TokenStream {
    request::make_path_type(input)
}

#[proc_macro_derive(Query)]
pub fn query(input: TokenStream) -> TokenStream {
    request::make_query_type(input)
}

#[proc_macro_attribute]
pub fn handler(args: TokenStream, input: TokenStream) -> TokenStream {
    handler::make_handler(args, input)
}

#[proc_macro_attribute]
pub fn middleware(args: TokenStream, input: TokenStream) -> TokenStream {
    middleware::make_middleware(args, input)
}

#[proc_macro_attribute]
pub fn job_factory(args: TokenStream, input: TokenStream) -> TokenStream {
    job::make_job(args, input)
}

#[proc_macro_attribute]
pub fn req_formatter(args: TokenStream, input: TokenStream) -> TokenStream {
    let expr_lit: ExprLit = parse(args).unwrap();
    let item_struct = parse_macro_input!(input as ItemStruct);
    match logger::make_req_fmt(expr_lit, item_struct) {
        Ok(r) => r,
        Err(e) => e.into_compile_error().into(),
    }
}

#[proc_macro_attribute]
pub fn resp_formatter(args: TokenStream, input: TokenStream) -> TokenStream {
    let expr_lit: ExprLit = parse(args).unwrap();
    let item_struct = parse_macro_input!(input as ItemStruct);
    match logger::make_res_fmt(expr_lit, item_struct) {
        Ok(r) => r,
        Err(e) => e.into_compile_error().into(),
    }
}

#[proc_macro]
pub fn app(input: TokenStream) -> TokenStream {
    let config = parse_macro_input!(input as app::Config);
    match app::make_app(config) {
        Ok(r) => r,
        Err(e) => e.into_compile_error().into(),
    }
}

enum HandlerArg {
    Handler(
        bool,
        Vec<proc_macro2::TokenStream>,
        Ident,
        proc_macro2::TokenStream,
        proc_macro2::TokenStream,
    ),
    Module(Ident, proc_macro2::TokenStream),
    Permanent(proc_macro2::TokenStream, proc_macro2::TokenStream),
}

fn make_handler_arg(
    tp: &PatType,
    i: u32,
    module_ident: &Ident,
    job_type: &String,
    name: &str,
) -> Result<HandlerArg, TokenStream> {
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

    let is_request = job_type == "Request";

    let attr = tp.attrs.first().unwrap();
    let attr_ident = attr.path.get_ident().unwrap();

    if let Type::Reference(rt) = *ttype.clone() {
        if let Type::Path(_) = *rt.elem.clone() {
            if attr_ident == "request_parts" {
                if !is_request {
                    return Err(Error::new_spanned(
                        attr_ident,
                        format!("request_parts only allowed for Request {}", name),
                    )
                    .to_compile_error()
                    .into());
                }
                let res = quote! {let #arg_name = p;};
                return Ok(HandlerArg::Permanent(arg_name.to_token_stream(), res));
            }
            if attr_ident == "body" {
                if !is_request {
                    return Err(Error::new_spanned(
                        attr_ident,
                        format!("body only allowed for Request {}", name),
                    )
                    .to_compile_error()
                    .into());
                }
                let res = quote! {let mut #arg_name = b;};
                let tt = quote! {&mut #arg_name};
                return Ok(HandlerArg::Permanent(tt, res));
            }
            if attr_ident == "response" {
                let res = quote! {let mut #arg_name = r;};
                let tt = quote! {&mut #arg_name};
                return Ok(HandlerArg::Permanent(tt, res));
            }
        }
    }

    if attr_ident == "handler" {
        let res = quote! {
            let #arg_name = ha
        };
        let mut bounds = vec![];
        if let Type::ImplTrait(imt) = *ttype.clone() {
            for j in imt.bounds {
                bounds.push(quote! {#j});
            }

            let ii = if let Pat::Ident(pi) = *tp.pat.clone() {
                pi.ident
            } else {
                format_ident!("Arg{}", i + 1)
            };

            let t_type = quote! {#ii};
            return Ok(HandlerArg::Handler(true, bounds, arg_name, t_type, res));
        }

        let t_type = quote! {#ttype};
        bounds.push(t_type.clone());
        return Ok(HandlerArg::Handler(false, bounds, arg_name, t_type, res));
    }
    if attr_ident == "inject" {
        let method_resolve = quote! {
            let #arg_name: #ttype = #module_ident.resolve();
        };
        return Ok(HandlerArg::Module(arg_name, method_resolve));
    }

    Err(Error::new_spanned(
        attr_ident,
        format!(
            "unsupported attribute #[{}] type {}",
            attr_ident,
            ttype.to_token_stream().to_string()
        ),
    )
    .to_compile_error()
    .into())
}
