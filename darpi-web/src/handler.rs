use crate::{Body, Response};
use async_trait::async_trait;
use futures::Future;
use hyper::Request;
use std::collections::HashMap;
use std::convert::Infallible;
use std::sync::Arc;

#[allow(dead_code)]
pub struct Args<'a, C> {
    pub request: Request<Body>,
    pub container: Arc<C>,
    pub route_args: HashMap<&'a str, &'a str>,
}

#[async_trait]
pub trait Handler<'a, C>
where
    C: 'static + Sync + Send,
{
    async fn call(self, args: Args<'a, C>) -> Result<Response<Body>, Infallible>;
}

#[async_trait]
impl<'a, C, F, Fut> Handler<'a, C> for F
where
    C: 'static + Sync + Send,
    F: FnOnce(Args<'a, C>) -> Fut + Sync + Send + 'static,
    Fut: Future<Output = Result<Response<Body>, Infallible>> + Sync + Send + 'static,
{
    async fn call(self, args: Args<'a, C>) -> Result<Response<Body>, std::convert::Infallible> {
        (self)(args).await
    }
}
