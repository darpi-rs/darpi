use crate::handler::MODULE_PREFIX;
use crate::{make_handler_arg, HandlerArg};
use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use quote::{ToTokens, TokenStreamExt};
use syn::{
    parse_macro_input, AttributeArgs, Error, FnArg, ItemFn, PathArguments, ReturnType, Type,
};

pub(crate) fn make_middleware(args: TokenStream, input: TokenStream) -> TokenStream {
    let mut func = parse_macro_input!(input as ItemFn);
    if func.sig.asyncness.is_none() {
        return Error::new_spanned(func, "Only Async functions can be used as middleware")
            .to_compile_error()
            .into();
    }

    let args = parse_macro_input!(args as AttributeArgs);

    if args.len() != 1 {
        return Error::new_spanned(func, format!("Expected 1 argument, {} given. Accepted arguments are middlewares type `Request` or `Response`", args.len()))
            .to_compile_error()
            .into();
    }

    let first_arg = args
        .first()
        .expect("this cannot happen")
        .to_token_stream()
        .to_string();

    let name = func.sig.ident.clone();
    let CallArgs {
        make,
        give,
        where_clause,
        handler_types,
        handler_bounds,
        handler_gen_types,
    } = match make_args(&mut func, &first_arg) {
        Ok(a) => a,
        Err(e) => return e,
    };

    let func_gen_params = &func.sig.generics.params;
    let func_gen_call = if !func_gen_params.is_empty() {
        quote! {::<#func_gen_params>}
    } else {
        Default::default()
    };

    let mut resolve_call = quote! {Self::#name#func_gen_call(#(#give ,)*).await};
    let mut k = Default::default();
    let mut e = Default::default();

    match &func.sig.output {
        ReturnType::Default => {
            resolve_call = quote! {
                #resolve_call;
                Ok(())
            };
            k = quote! {()};
            e = quote! {std::convert::Infallible};
        }
        ReturnType::Type(_, t) => {
            if let Type::Path(tp) = *t.clone() {
                let last = tp.path.segments.last().unwrap();
                if let PathArguments::AngleBracketed(ab) = &last.arguments {
                    if last.ident == "Result" {
                        assert_eq!(ab.args.len(), 2);
                        k = ab.args[0].to_token_stream();
                        e = ab.args[1].to_token_stream();
                    } else if last.ident == "Option" {
                        assert_eq!(ab.args.len(), 1);
                        k = ab.args[0].to_token_stream();
                        e = quote! {std::convert::Infallible};
                    }
                }
            }
        }
    }

    if e.is_empty() {
        panic!("cannot find error type")
    }

    if k.is_empty() {
        panic!("cannot find type")
    }

    let where_module = match where_clause.is_empty() {
        true => Default::default(),
        false => {
            quote! {+ #(#where_clause +)*}
        }
    };

    let handler_t = if handler_types.len() == 1 {
        quote! {#(#handler_types)*}
    } else {
        quote! {( #(#handler_types ,)* )}
    };

    let (gen_params, with_brackets, bounds, phantom_data) = if handler_bounds.is_empty() {
        (
            Default::default(),
            Default::default(),
            Default::default(),
            quote! {;},
        )
    } else {
        let mut bound = vec![];
        let mut phantom_data = vec![];

        for i in 0..handler_bounds.len() {
            if let Some(id) = handler_gen_types.get(i) {
                let hb = handler_bounds[i].clone();
                bound.push(quote! {#id: #(#hb +)*});
                let m_id = format_ident!("_marker{}", i);
                phantom_data.push(quote! {#m_id: std::marker::PhantomData<#id>});
            }
        }

        (
            quote! {, #(#handler_gen_types ,)*},
            quote! {<#(#handler_gen_types ,)*>},
            quote! { #(#bound ,)*},
            quote! {{#(#phantom_data ,)*}},
        )
    };

    let tokens = match first_arg.as_str() {
        "Request" => {
            quote! {
                #[allow(non_camel_case_types, missing_docs)]
                pub struct #name#with_brackets#phantom_data
                #[allow(non_camel_case_types, missing_docs)]
                impl#with_brackets #name#with_brackets {
                    #func
                }

                #[allow(non_camel_case_types, missing_docs)]
                #[darpi::async_trait]
                impl<M #gen_params> darpi::RequestMiddleware<M> for #name#with_brackets
                where
                    M: 'static + Sync + Send #where_module,
                    #bounds
                {
                    type HandlerArgs = #handler_t;
                    type Error = #e;
                    type Type = #k;

                    async fn call(
                        r: &mut darpi::Request<Body>,
                        module: std::sync::Arc<M>,
                        ha: Self::HandlerArgs,
                    ) -> Result<Self::Type, Self::Error> {
                        #(#make )*
                        #resolve_call
                    }
                }
            }
        }
        "Response" => {
            quote! {
                #[allow(non_camel_case_types, missing_docs)]
                pub struct #name#with_brackets#phantom_data
                #[allow(non_camel_case_types, missing_docs)]
                impl#with_brackets #name#with_brackets {
                    #func
                }

                #[allow(non_camel_case_types, missing_docs)]
                #[darpi::async_trait]
                impl<M #gen_params> darpi::ResponseMiddleware<M> for #name#with_brackets
                where
                    M: 'static + Sync + Send #where_module,
                    #bounds
                {
                    type HandlerArgs = #handler_t;
                    type Error = #e;
                    type Type = #k;

                    async fn call(
                        r: &mut darpi::Response<Body>,
                        module: std::sync::Arc<M>,
                        ha: Self::HandlerArgs,
                    ) -> Result<Self::Type, Self::Error> {
                        #(#make )*
                        #resolve_call
                    }
                }
            }
        }
        _ => Error::new_spanned(
            func,
            format!(
                "Accepted arguments are middlewares type `Request` or `Response`, `{}` given",
                first_arg
            ),
        )
        .to_compile_error()
        .into(),
    };

    //panic!("{}", tokens.to_string());
    tokens.into()
}

struct CallArgs {
    make: Vec<TokenStream2>,
    give: Vec<TokenStream2>,
    where_clause: Vec<TokenStream2>,
    handler_types: Vec<TokenStream2>,
    handler_bounds: Vec<Vec<TokenStream2>>,
    handler_gen_types: Vec<TokenStream2>,
}

fn make_args(func: &mut ItemFn, middleware_type: &String) -> Result<CallArgs, TokenStream> {
    let mut make = vec![];
    let mut give = vec![];
    let mut i = 0_u32;
    let mut where_clause = vec![];
    let mut handler_types = vec![];
    let mut handler_gen_types = vec![];
    let mut handler_bounds = vec![];
    let mut handler_make = vec![];

    let module_ident = format_ident!("{}", MODULE_PREFIX);

    for arg in func.sig.inputs.iter_mut() {
        if let FnArg::Typed(tp) = arg {
            let h_args = match make_handler_arg(tp, i, &module_ident, middleware_type, "middleware")
            {
                Ok(k) => k,
                Err(e) => return Err(e),
            };
            let (is_h, arg_name, method_resolve) = match h_args {
                HandlerArg::Permanent(i, ts) => (false, i, ts),
                HandlerArg::Handler(is_gen, bounds, id, ttype, ts) => {
                    if is_gen {
                        handler_gen_types.push(ttype.clone());
                    }
                    handler_types.push(ttype);
                    if !bounds.is_empty() {
                        handler_bounds.push(bounds);
                    }
                    (true, id.to_token_stream(), ts)
                }
                HandlerArg::Module(i, ts) => {
                    if let Type::Path(tp) = *tp.ty.clone() {
                        let last = tp.path.segments.last().expect("PathSegment");
                        let args = &last.arguments;
                        if let PathArguments::AngleBracketed(ab) = args {
                            let args = &ab.args;
                            where_clause.push(quote! {shaku::HasComponent<#args>});
                        }
                    }
                    (false, i.to_token_stream(), ts)
                }
            };

            if is_h {
                handler_make.push(method_resolve);
            } else {
                make.push(method_resolve);
            }
            give.push(quote! {#arg_name});
            i += 1;
            tp.attrs = Default::default();
        }
    }

    if handler_make.len() != 1 {
        handler_make.iter_mut().enumerate().for_each(|(i, hm)| {
            let ii = syn::Index::from(i);
            hm.append_all(quote! {.#ii});
        });
    }

    handler_make.iter_mut().for_each(|hm| {
        hm.append_all(quote! {;});
    });

    make.append(&mut handler_make);

    Ok(CallArgs {
        make,
        give,
        where_clause,
        handler_types,
        handler_bounds,
        handler_gen_types,
    })
}
