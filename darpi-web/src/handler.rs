use crate::{Body, Response};
use async_trait::async_trait;
use http::request::Parts as RequestParts;
use std::collections::HashMap;
use std::convert::Infallible;
use std::sync::Arc;

#[allow(dead_code)]
pub struct Args<'a, C> {
    pub request_parts: &'a mut RequestParts,
    pub container: Arc<C>,
    pub body: Body,
    pub route_args: HashMap<&'a str, &'a str>,
}

#[async_trait]
pub trait Handler<'a, C>
where
    C: 'static + Sync + Send,
{
    async fn call(&self, args: Args<'a, C>) -> Result<Response<Body>, Infallible>;
}

#[async_trait]
impl<'a, C, F> Handler<'a, C> for F
where
    C: 'static + Sync + Send,
    F: Fn(Args<'a, C>) -> Result<Response<Body>, Infallible> + 'static + Send + Sync,
{
    async fn call(&self, args: Args<'a, C>) -> Result<Response<Body>, std::convert::Infallible> {
        (self)(args)
    }
}
