use crate::request::FromRequestBody;
use crate::response::{Responder, ResponderError};
use crate::Response;
use async_trait::async_trait;
use bytes::Buf;
use derive_more::Display;
use http::header::HeaderName;
use http::{header, HeaderMap, HeaderValue};
use hyper::Body;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Deserializer, Serialize};
use serde_xml_rs::Error;
use std::{fmt, ops};

pub struct Xml<T> {
    t: T,
    hm: HeaderMap,
}

impl<T> Xml<T> {
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

    async fn deserialize_future(b: Body) -> Result<Xml<T>, XmlErr>
    where
        T: DeserializeOwned,
    {
        let full_body = hyper::body::to_bytes(b).await?;
        let ser: T = serde_xml_rs::from_reader(full_body.reader())?;
        Ok(Xml::new(ser))
    }
}

#[async_trait]
impl<T> FromRequestBody<Xml<T>, XmlErr> for Xml<T>
where
    T: DeserializeOwned + 'static,
{
    async fn assert_content_type(content_type: Option<&HeaderValue>) -> Result<(), XmlErr> {
        if let Some(hv) = content_type {
            if hv != "application/xml" {
                return Err(XmlErr::InvalidContentType);
            }
            return Ok(());
        }
        Err(XmlErr::MissingContentType)
    }
    async fn extract(_: &HeaderMap, b: Body) -> Result<Xml<T>, XmlErr> {
        Self::deserialize_future(b).await
    }
}

impl<'de, T> Deserialize<'de> for Xml<T>
where
    T: DeserializeOwned,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, <D as Deserializer<'de>>::Error>
    where
        D: Deserializer<'de>,
    {
        let deser = T::deserialize(deserializer)?.into();
        Ok(Xml::new(deser))
    }
}

impl<T> ops::Deref for Xml<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.t
    }
}

impl<T> ops::DerefMut for Xml<T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.t
    }
}

impl<T> fmt::Debug for Xml<T>
where
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Xml: {:?}", self.t)
    }
}

impl<T> fmt::Display for Xml<T>
where
    T: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.t, f)
    }
}

impl<T> Responder for Xml<T>
where
    T: Serialize,
{
    fn respond(self) -> Response<Body> {
        match serde_xml_rs::to_string(&self.t) {
            Ok(body) => {
                let mut rb = Response::builder()
                    .header(header::CONTENT_TYPE, "application/xml")
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
pub enum XmlErr {
    ReadBody(hyper::Error),
    Serde(Error),
    InvalidContentType,
    MissingContentType,
}

impl From<Error> for XmlErr {
    fn from(e: Error) -> Self {
        Self::Serde(e)
    }
}

impl From<hyper::Error> for XmlErr {
    fn from(e: hyper::Error) -> Self {
        Self::ReadBody(e)
    }
}

impl ResponderError for XmlErr {}
impl ResponderError for serde_xml_rs::Error {}
