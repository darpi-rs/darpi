use async_channel::{bounded, Receiver, Sender};
use async_graphql::http::MultipartOptions;
use async_graphql::{ParseRequestError, Result};
use async_trait::async_trait;
use darpi::header::HeaderValue;
use darpi::request::{FromRequestBodyWithContainer, QueryPayloadError};
use darpi::{body::Bytes, header, hyper, response::ResponderError, Body, Query, StatusCode};
use derive_more::Display;
use futures_util::{StreamExt, TryStreamExt};
use http::HeaderMap;
use serde::{de::DeserializeOwned, Deserialize, Deserializer};
use serde_json;
use shaku::{Component, HasComponent, Interface};
use std::sync::Arc;

#[derive(Debug, Deserialize, Query)]
pub struct BatchRequest(pub async_graphql::BatchRequest);

impl BatchRequest {
    #[must_use]
    pub fn into_inner(self) -> async_graphql::BatchRequest {
        self.0
    }
}

#[derive(Debug, Deserialize)]
pub struct Response(pub async_graphql::Response);

impl darpi::response::Responder for Response {
    fn respond(self) -> darpi::Response<darpi::Body> {
        let mut res = darpi::Response::builder()
            .header(header::CONTENT_TYPE, "application/json")
            .status(StatusCode::OK)
            .body(darpi::Body::from(serde_json::to_string(&self.0).unwrap()))
            .unwrap();

        if self.0.is_ok() {
            if let Some(cache_control) = self.0.cache_control.value() {
                res.headers_mut()
                    .insert("cache-control", cache_control.parse().unwrap());
            }
            for (name, value) in self.0.http_headers {
                if let Some(header_name) = name {
                    if let Ok(val) = HeaderValue::from_str(&value) {
                        res.headers_mut().insert(header_name, val);
                    }
                }
            }
        }
        res
    }
}

impl From<async_graphql::Response> for Response {
    fn from(r: async_graphql::Response) -> Self {
        Self(r)
    }
}

pub struct GraphQLBody<T>(pub T);

impl darpi::response::ErrResponder<darpi::request::QueryPayloadError, darpi::Body>
    for GraphQLBody<Request>
{
    fn respond_err(e: QueryPayloadError) -> darpi::Response<Body> {
        Request::respond_err(e)
    }
}

#[derive(Display)]
pub enum GraphQLError {
    ParseRequest(ParseRequestError),
    Hyper(hyper::Error),
}

impl From<ParseRequestError> for GraphQLError {
    fn from(e: ParseRequestError) -> Self {
        Self::ParseRequest(e)
    }
}

impl From<hyper::Error> for GraphQLError {
    fn from(e: hyper::Error) -> Self {
        Self::Hyper(e)
    }
}

impl ResponderError for GraphQLError {}

impl<'de, T> Deserialize<'de> for GraphQLBody<T>
where
    T: DeserializeOwned,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, <D as Deserializer<'de>>::Error>
    where
        D: Deserializer<'de>,
    {
        let deser = T::deserialize(deserializer)?.into();
        Ok(GraphQLBody(deser))
    }
}

pub trait MultipartOptionsProvider: Interface {
    fn get(&self) -> MultipartOptions;
}

#[derive(Component)]
#[shaku(interface = MultipartOptionsProvider)]
pub struct MultipartOptionsProviderImpl {
    opts: MultipartOptions,
}

impl MultipartOptionsProvider for MultipartOptionsProviderImpl {
    fn get(&self) -> MultipartOptions {
        self.opts.clone()
    }
}

#[async_trait]
impl<C: 'static> FromRequestBodyWithContainer<GraphQLBody<BatchRequest>, GraphQLError, C>
    for GraphQLBody<BatchRequest>
where
    C: HasComponent<dyn MultipartOptionsProvider>,
{
    async fn extract(
        headers: &HeaderMap,
        mut body: darpi::Body,
        container: Arc<C>,
    ) -> Result<GraphQLBody<BatchRequest>, GraphQLError> {
        let content_type = headers
            .get(http::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .map(|value| value.to_string());

        let (mut tx, rx): (
            Sender<std::result::Result<Bytes, _>>,
            Receiver<std::result::Result<Bytes, _>>,
        ) = bounded(16);

        tokio::runtime::Handle::current().spawn(async move {
            while let Some(item) = body.next().await {
                if tx.send(item).await.is_err() {
                    return;
                }
            }
        });

        let opts = container.resolve().get();
        Ok(GraphQLBody(BatchRequest(
            async_graphql::http::receive_batch_body(
                content_type,
                rx.map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))
                    .into_async_read(),
                opts,
            )
            .await
            .map_err(|e| GraphQLError::ParseRequest(e))?,
        )))
    }
}

#[derive(Debug, Deserialize, Query)]
pub struct Request(pub async_graphql::Request);

impl Request {
    #[must_use]
    pub fn into_inner(self) -> async_graphql::Request {
        self.0
    }
}

#[async_trait]
impl<C: 'static> FromRequestBodyWithContainer<GraphQLBody<Request>, GraphQLError, C>
    for GraphQLBody<Request>
where
    C: HasComponent<dyn MultipartOptionsProvider>,
{
    async fn extract(
        headers: &HeaderMap,
        body: darpi::Body,
        container: Arc<C>,
    ) -> Result<GraphQLBody<Request>, GraphQLError> {
        let res: GraphQLBody<BatchRequest> = GraphQLBody::extract(headers, body, container).await?;

        Ok(res
            .0
            .into_inner()
            .into_single()
            .map(|r| GraphQLBody(Request(r)))
            .map_err(|e| GraphQLError::ParseRequest(e))?)
    }
}
