use crate::{response::ResponderError, Body, Request, Response};
use async_trait::async_trait;
use std::sync::Arc;

#[async_trait]
pub trait RequestMiddleware<M>
where
    M: 'static + Sync + Send,
{
    type HandlerArgs: 'static + Sync + Send;
    type Error: ResponderError;
    type Type;

    async fn call(
        req: &mut Request<Body>,
        module: Arc<M>,
        ha: Self::HandlerArgs,
    ) -> Result<Self::Type, Self::Error>;
}

#[async_trait]
pub trait ResponseMiddleware<M>
where
    M: 'static + Sync + Send,
{
    type HandlerArgs;
    type Error: ResponderError;
    type Type;
    async fn call(
        r: &mut Response<Body>,
        module: Arc<M>,
        ha: Self::HandlerArgs,
    ) -> Result<Self::Type, Self::Error>;
}
