use crate::request::FromRequestBody;
use crate::response::{Responder, ResponderError};
use crate::Response;
use async_trait::async_trait;
use derive_more::Display;
use http::header::HeaderName;
use http::{header, HeaderMap, HeaderValue};
use hyper::Body;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Deserializer, Serialize};
use serde_yaml::Error;
use std::{fmt, ops};

pub struct Yaml<T> {
    t: T,
    hm: HeaderMap,
}

impl<T> Yaml<T> {
    pub fn new(t: T) -> Self {
        Self {
            t,
            hm: Default::default(),
        }
    }

    pub fn header(mut self, key: HeaderName, value: HeaderValue) -> Self {
        self.hm.append(key, value);
        self
    }

    async fn deserialize_future(b: Body) -> Result<Yaml<T>, YamlErr>
    where
        T: DeserializeOwned,
    {
        let full_body = hyper::body::to_bytes(b).await?;
        let ser: T = serde_yaml::from_slice(&full_body)?;
        Ok(Yaml::new(ser))
    }
}

#[async_trait]
impl<T> FromRequestBody<Yaml<T>, YamlErr> for Yaml<T>
where
    T: DeserializeOwned + 'static,
{
    async fn assert_content_type(content_type: Option<&HeaderValue>) -> Result<(), YamlErr> {
        if let Some(hv) = content_type {
            if hv != "application/yaml" && hv != "text/yaml" {
                return Err(YamlErr::InvalidContentType);
            }
            return Ok(());
        }
        Err(YamlErr::MissingContentType)
    }
    async fn extract(_: &HeaderMap, b: Body) -> Result<Yaml<T>, YamlErr> {
        Self::deserialize_future(b).await
    }
}

impl<'de, T> Deserialize<'de> for Yaml<T>
where
    T: DeserializeOwned,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, <D as Deserializer<'de>>::Error>
    where
        D: Deserializer<'de>,
    {
        let deser = T::deserialize(deserializer)?.into();
        Ok(Yaml::new(deser))
    }
}

impl<T> ops::Deref for Yaml<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.t
    }
}

impl<T> ops::DerefMut for Yaml<T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.t
    }
}

impl<T> fmt::Debug for Yaml<T>
where
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Yaml: {:?}", self.t)
    }
}

impl<T> fmt::Display for Yaml<T>
where
    T: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.t, f)
    }
}

impl<T> Responder for Yaml<T>
where
    T: Serialize,
{
    fn respond(self) -> Response<Body> {
        match serde_yaml::to_string(&self.t) {
            Ok(body) => {
                let mut rb = Response::builder()
                    .header(header::CONTENT_TYPE, "application/Yaml")
                    .status(self.status_code());

                for (hk, hv) in self.hm.iter() {
                    rb = rb.header(hk, hv);
                }

                rb.body(Body::from(body)).expect("this cannot happen")
            }
            Err(e) => e.respond_err(),
        }
    }
}

#[derive(Display)]
pub enum YamlErr {
    ReadBody(hyper::Error),
    Serde(Error),
    InvalidContentType,
    MissingContentType,
}

impl From<Error> for YamlErr {
    fn from(e: Error) -> Self {
        Self::Serde(e)
    }
}

impl From<hyper::Error> for YamlErr {
    fn from(e: hyper::Error) -> Self {
        Self::ReadBody(e)
    }
}

impl ResponderError for YamlErr {}
impl ResponderError for serde_yaml::Error {}
