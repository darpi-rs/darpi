use crate::{Body, Response};
use async_trait::async_trait;
use futures::future::BoxFuture;
use futures::{Future, FutureExt, SinkExt, StreamExt};
use hyper::upgrade::{on, Upgraded};
use hyper::Request;
use std::collections::HashMap;
use std::convert::Infallible;
use std::sync::Arc;
use tokio_tungstenite::{accept_async, tungstenite::Error};

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

async fn asd<'a, C: Send + Sync + 'static>(
    mut req: Request<Body>,
) -> Result<Response<Body>, Infallible> {
    let upgrade = on(req).await;
    if let Ok(upg) = upgrade {
        handle_connection(upg).await;
    }

    unimplemented!()
}

async fn handle_connection(stream: Upgraded) {
    let mut ws_stream = accept_async(stream).await.expect("Failed to accept");

    while let Some(msg) = ws_stream.next().await {
        let msg = msg.unwrap();
        if msg.is_text() || msg.is_binary() {
            ws_stream.send(msg).await.unwrap();
        }
    }
}
