use crate::response;
use crate::response::ResponderError;
use async_trait::async_trait;
use derive_more::{Display, From};
use http::{HeaderMap, HeaderValue};
use hyper::Body;
use hyper::Response;
use serde::de;
use serde_urlencoded;
use std::sync::Arc;

#[async_trait]
pub trait FromRequestBodyWithContainer<T, E, C>
where
    T: de::DeserializeOwned + 'static,
    E: ResponderError + 'static,
    C: 'static + Sync + Send,
{
    async fn assert_content_type(_content_type: Option<&HeaderValue>, _: Arc<C>) -> Result<(), E> {
        Ok(())
    }
    async fn extract(headers: &HeaderMap, b: Body, _: Arc<C>) -> Result<T, E>;
}

#[async_trait]
pub trait FromRequestBody<T, E>
where
    T: de::DeserializeOwned + 'static,
    E: ResponderError + 'static,
{
    async fn assert_content_type(_content_type: Option<&HeaderValue>) -> Result<(), E> {
        Ok(())
    }
    async fn extract(headers: &HeaderMap, b: Body) -> Result<T, E>;
}

#[async_trait]
impl<F, T, E, C> FromRequestBodyWithContainer<T, E, C> for F
where
    F: FromRequestBody<T, E> + 'static,
    T: de::DeserializeOwned + 'static,
    E: ResponderError + 'static,
    C: std::any::Any + Sync + Send,
{
    async fn assert_content_type(content_type: Option<&HeaderValue>, _: Arc<C>) -> Result<(), E> {
        F::assert_content_type(content_type).await
    }
    async fn extract(headers: &HeaderMap<HeaderValue>, b: Body, _: Arc<C>) -> Result<T, E> {
        F::extract(headers, b).await
    }
}

#[derive(Debug, Display, From)]
pub enum RequestErr {
    #[display(fmt = "Not found")]
    NotFound,
}

impl ResponderError for RequestErr {}

/// A set of errors that can occur during parsing query strings
#[derive(Debug, Display, From)]
pub enum PayloadError {
    /// Deserialize error
    #[display(fmt = "Payload deserialize error: {}", _0)]
    Deserialize(serde::de::value::Error),
    #[display(fmt = "Empty Payload")]
    NotExist,
    #[display(fmt = "Payload maximum {} exceeded: received {} bytes", _0, _1)]
    Size(u64, u64),
}

impl ResponderError for PayloadError {}

/// A set of errors that can occur during parsing query strings
#[derive(Debug, Display, From)]
pub enum QueryPayloadError {
    /// Deserialize error
    #[display(fmt = "Query deserialize error: {}", _0)]
    Deserialize(serde::de::value::Error),
    #[display(fmt = "Empty query")]
    NotExist,
}

impl ResponderError for QueryPayloadError {}
impl std::error::Error for QueryPayloadError {}

pub trait FromQuery<T, E> {
    fn from_query(query_str: Option<&str>) -> Result<T, E>
    where
        T: de::DeserializeOwned,
        E: ResponderError;
}

impl<T> FromQuery<T, QueryPayloadError> for T {
    fn from_query(query_str: Option<&str>) -> Result<T, QueryPayloadError>
    where
        T: de::DeserializeOwned,
    {
        match query_str {
            Some(query_str) => serde_urlencoded::from_str::<T>(query_str)
                .map(|val| Ok(val))
                .unwrap_or_else(move |e| Err(QueryPayloadError::Deserialize(e))),
            None => Err(QueryPayloadError::NotExist),
        }
    }
}

#[derive(Debug, Display, From)]
pub enum PathError {
    #[display(fmt = "Path deserialize error: {}", _0)]
    Deserialize(String),
}

impl ResponderError for PathError {}
impl std::error::Error for PathError {}

pub fn assert_respond_err<T, E>(e: E) -> Response<Body>
where
    T: response::ErrResponder<E, Body>,
    E: std::error::Error,
{
    T::respond_err(e)
}

impl<T> FromQuery<Option<T>, QueryPayloadError> for T
where
    T: de::DeserializeOwned,
{
    fn from_query(query_str: Option<&str>) -> Result<Option<T>, QueryPayloadError>
    where
        T: FromQuery<T, QueryPayloadError>,
    {
        match T::from_query(query_str) {
            Ok(t) => Ok(Some(t)),
            Err(_) => Ok(None),
        }
    }
}
