use crate::{Body, Response};
use async_trait::async_trait;
use futures::Future;
use hyper::Request;
use std::convert::Infallible;
use std::sync::Arc;

#[allow(dead_code)]
pub struct Args<C, A> {
    pub request: Request<Body>,
    pub container: Arc<C>,
    pub route_args: A,
}

#[async_trait]
pub trait Handler<C, A>
where
    C: 'static + Sync + Send,
{
    async fn call(self, args: Args<C, A>) -> Result<Response<Body>, Infallible>;
}

#[async_trait]
impl<C, F, Fut, A> Handler<C, A> for F
where
    C: 'static + Sync + Send,
    F: FnOnce(Args<C, A>) -> Fut + Sync + Send + 'static,
    Fut: Future<Output = Result<Response<Body>, Infallible>> + Sync + Send + 'static,
    A: 'static + Sync + Send,
{
    async fn call(self, args: Args<C, A>) -> Result<Response<Body>, std::convert::Infallible> {
        (self)(args).await
    }
}
