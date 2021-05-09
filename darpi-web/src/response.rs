use crate::ws;
use bytes::BytesMut;
use http::{header, HeaderValue};
use hyper::header::SEC_WEBSOCKET_KEY;
use hyper::{Body, Error, HeaderMap, Response, StatusCode};
use std::convert::Infallible;
use std::io::Write;
use std::{fmt, io};

pub trait Responder {
    fn status_code(&self) -> StatusCode {
        StatusCode::OK
    }
    fn respond(self) -> Response<Body>;
}

#[derive(Debug)]
pub struct UpgradeWS {
    key: HeaderValue,
}

impl UpgradeWS {
    pub fn from_header(headers: &HeaderMap) -> Option<Self> {
        let key = headers.get(SEC_WEBSOCKET_KEY)?;
        Some(Self { key: key.clone() })
    }
}

impl Responder for UpgradeWS {
    fn respond(self) -> Response<Body> {
        Response::builder()
            .status(StatusCode::SWITCHING_PROTOCOLS)
            .header(header::UPGRADE, "websocket")
            .header(header::CONNECTION, "upgrade")
            .header(
                header::SEC_WEBSOCKET_ACCEPT,
                ws::convert_key(self.key.as_bytes()),
            )
            .body(Body::empty())
            .unwrap()
    }
}

impl Responder for &'static str {
    fn respond(self) -> Response<Body> {
        Response::builder()
            .header(header::CONTENT_TYPE, "text/plain; charset=utf-8")
            .status(StatusCode::OK)
            .body(Body::from(self))
            .unwrap()
    }
}

impl Responder for &'static [u8] {
    fn respond(self) -> Response<Body> {
        Response::builder()
            .header(header::CONTENT_TYPE, "application/octet-stream")
            .status(StatusCode::OK)
            .body(Body::from(self))
            .unwrap()
    }
}

impl Responder for String {
    fn respond(self) -> Response<Body> {
        Response::builder()
            .header(header::CONTENT_TYPE, "text/plain; charset=utf-8")
            .status(StatusCode::OK)
            .body(Body::from(self))
            .unwrap()
    }
}

macro_rules! impl_responder_n {
    ($($x:ty),* $(,)*) => {
        $(
            impl Responder for $x {
                fn respond(self) -> Response<Body> {
                    Response::builder()
                        .header(header::CONTENT_TYPE, "text/plain; charset=utf-8")
                        .status(StatusCode::OK)
                        .body(Body::from(format!("{}", self)))
                        .unwrap()
                }
            }
        )*
    };
}

impl_responder_n!(u8, u16, u32, u64, u128, i8, i16, i32, i64, i128);

impl Responder for () {
    fn respond(self) -> Response<Body> {
        Response::builder()
            .header(header::CONTENT_TYPE, "text/plain; charset=utf-8")
            .status(StatusCode::OK)
            .body(Body::empty())
            .unwrap()
    }
}

impl<T> Responder for Option<T>
where
    T: Responder,
{
    fn respond(self) -> Response<Body> {
        match self {
            Some(t) => t.respond(),
            None => Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::empty())
                .unwrap(),
        }
    }
}

impl<T, E> Responder for Result<T, E>
where
    E: ResponderError,
    T: Responder,
{
    fn respond(self) -> Response<Body> {
        match self {
            Ok(t) => t.respond(),
            Err(e) => e.respond_err(),
        }
    }
}

impl Responder for Response<Body> {
    #[inline(always)]
    fn respond(self) -> Response<Body> {
        self
    }
}

pub trait ResponderError: fmt::Display {
    fn status_code(&self) -> StatusCode {
        StatusCode::INTERNAL_SERVER_ERROR
    }
    fn respond_err(&self) -> Response<Body> {
        let mut buf = BytesMut::new();
        let _ = write!(ByteWriter(&mut buf), "{}", self);
        let b = buf.freeze();

        Response::builder()
            .header(header::CONTENT_TYPE, "text/plain; charset=utf-8")
            .status(self.status_code())
            .body(Body::from(b))
            .expect("this cannot happen")
    }
}

impl ResponderError for tokio::task::JoinError {}

struct ByteWriter<'a>(pub &'a mut BytesMut);

impl<'a> Write for ByteWriter<'a> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl ResponderError for Infallible {}
impl ResponderError for String {}
impl ResponderError for &str {}
impl ResponderError for Error {}
impl ResponderError for dyn std::error::Error {}

pub trait ErrResponder<E, B>
where
    E: std::error::Error,
{
    fn respond_err(e: E) -> Response<B>;
}

impl<T> ErrResponder<crate::request::QueryPayloadError, Body> for Option<T> {
    fn respond_err(e: crate::request::QueryPayloadError) -> Response<Body> {
        let msg = match e {
            crate::request::QueryPayloadError::Deserialize(de) => de.to_string(),
            crate::request::QueryPayloadError::NotExist => "missing query params".to_string(),
        };

        Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Body::from(msg))
            .expect("this not to happen!")
    }
}
