use proc_macro::TokenStream;
use proc_macro2::Ident;
use quote::quote;
use syn::parse::Parse;
use syn::{braced, parse::ParseStream, parse_macro_input, token, Error, Result as SynResult};

pub(crate) fn main(args: TokenStream, item: TokenStream, is_test: bool) -> TokenStream {
    let mut input = syn::parse_macro_input!(item as syn::ItemFn);

    if input.sig.asyncness.take().is_none() {
        let msg = "the `async` keyword is missing from the function declaration";
        return syn::Error::new_spanned(input.sig.fn_token, msg)
            .to_compile_error()
            .into();
    }

    if input.sig.ident == "main" && !input.sig.inputs.is_empty() {
        let msg = "the main function cannot accept arguments";
        return syn::Error::new_spanned(&input.sig.ident, msg)
            .to_compile_error()
            .into();
    }

    let (async_, blocking, cpu) = if args.is_empty() {
        (None, None, None)
    } else {
        let args = parse_macro_input!(args as Config);
        if let Some(threads) = args.threads {
            (threads.async_, threads.blocking, threads.cpu)
        } else {
            (None, None, None)
        }
    };

    let num_threads = cpu.map_or(Default::default(), |a| {
        quote! {
           .num_threads(#a)
        }
    });

    let max_blocking_threads = blocking.map_or(Default::default(), |a| {
        quote! {
           .max_blocking_threads(#a)
        }
    });

    let async_threads = async_.map_or(Default::default(), |a| {
        quote! {
           .worker_threads(#a)
        }
    });

    let block: syn::ExprBlock = syn::parse2(quote! {
        {
            static __ONCE_INTERNAL__: std::sync::Once = std::sync::Once::new();

            __ONCE_INTERNAL__.call_once(|| {
                darpi::rayon::ThreadPoolBuilder::new()
                #num_threads
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
        }
    })
    .unwrap();

    let mut stmt: Vec<syn::Stmt> = vec![syn::Stmt::Expr(syn::Expr::Block(block))];

    stmt.extend(input.block.stmts.clone());
    input.block.stmts = stmt;

    let body = &input.block;
    let brace_token = input.block.brace_token;

    input.block = syn::parse2(quote! {
         {
            tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                #max_blocking_threads
                #async_threads
                .build()
                .unwrap()
                .block_on(async #body)
        }
    })
    .unwrap();
    input.block.brace_token = brace_token;

    let header = if is_test {
        quote! {
            #[::core::prelude::v1::test]
        }
    } else {
        quote! {}
    };

    let result = quote! {
        #header
        #input
    };

    result.into()
}

#[derive(Debug)]
pub struct Config {
    pub(crate) threads: Option<Threads>,
}

#[derive(Debug)]
pub struct Threads {
    pub(crate) async_: Option<syn::LitInt>,
    pub(crate) blocking: Option<syn::LitInt>,
    pub(crate) cpu: Option<syn::LitInt>,
}

impl Parse for Threads {
    fn parse(input: ParseStream) -> SynResult<Self> {
        let content;
        let _ = braced!(content in input);

        let mut async_threads: Option<syn::LitInt> = None;
        let mut blocking_threads: Option<syn::LitInt> = None;
        let mut cpu_threads: Option<syn::LitInt> = None;

        while !content.is_empty() {
            if content.peek(token::Comma) {
                let _: token::Comma = content.parse()?;
            }

            let key = content.fork().parse::<Ident>()?;

            if key == "async" {
                let _: Ident = content.parse()?;
                let _: token::Colon = content.parse()?;
                let async_: syn::LitInt = content.parse()?;
                async_threads = Some(async_);
                continue;
            }
            if key == "max_blocking" {
                let _: Ident = content.parse()?;
                let _: token::Colon = content.parse()?;
                let blocking: syn::LitInt = content.parse()?;
                blocking_threads = Some(blocking);
                continue;
            }
            if key == "max_cpu" {
                let _: Ident = content.parse()?;
                let _: token::Colon = content.parse()?;
                let cpu: syn::LitInt = content.parse()?;
                cpu_threads = Some(cpu);
                continue;
            }

            return Err(Error::new_spanned(
                key.clone(),
                format!(
                    "unknown thread configuration key: `{}`. Only `async`, `blocking` and `cpu` are allowed",
                    key
                ),
            ));
        }

        return Ok(Threads {
            async_: async_threads,
            blocking: blocking_threads,
            cpu: cpu_threads,
        });
    }
}

impl Parse for Config {
    fn parse(input: ParseStream) -> SynResult<Self> {
        let content;
        let _ = braced!(content in input);

        let mut threads: Option<Threads> = None;

        while !content.is_empty() {
            if content.peek(token::Comma) {
                let _: token::Comma = content.parse()?;
            }

            let key = content.fork().parse::<Ident>()?;

            if key == "threads" {
                let _: Ident = content.parse()?;
                let _: token::Colon = content.parse()?;

                let tr: Threads = content.parse()?;
                threads = Some(tr);
                continue;
            }

            return Err(Error::new_spanned(
                key.clone(),
                format!(
                    "unknown main configuration key: `{}`. Only `threads` is allowed",
                    key
                ),
            ));
        }

        return Ok(Config { threads });
    }
}
