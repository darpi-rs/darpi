use darpi::response::ResponderError;
use darpi::{handler, Path};
#[cfg(test)]
use darpi::{Args, Body, Handler, Request, StatusCode};
use darpi_web::Json;
use derive_more::Display;
use http::header::HeaderName;
use http::HeaderValue;
use serde::{Deserialize, Serialize};
use std::convert::TryInto;
use std::num::TryFromIntError;
use std::str::FromStr;
#[cfg(test)]
use std::sync::Arc;

#[darpi::test]
async fn increment_byte_ok() {
    let req = Request::get("http://127.0.0.1:3000/increment_bute/5")
        .body(Body::empty())
        .unwrap();

    let resp = Handler::call(
        increment_byte,
        Args {
            request: req,
            container: Arc::new(()),
            route_args: ("5".to_string(),),
        },
    )
    .await
    .unwrap();

    assert_eq!(StatusCode::OK, resp.status());
}

#[tokio::test]
async fn increment_byte_not_ok() {
    let req = Request::get("http://127.0.0.1:3000/increment_bute/5")
        .body(Body::empty())
        .unwrap();

    let resp = Handler::call(
        increment_byte,
        Args {
            request: req,
            container: Arc::new(()),
            route_args: ("255".to_string(),),
        },
    )
    .await
    .unwrap();

    assert_eq!(StatusCode::INTERNAL_SERVER_ERROR, resp.status());
}

#[tokio::test]
async fn set_correct_header() {
    let req = Request::get("http://127.0.0.1:3000/json")
        .body(Body::empty())
        .unwrap();

    let resp = Handler::call(
        json,
        Args {
            request: req,
            container: Arc::new(()),
            route_args: (),
        },
    )
    .await
    .unwrap();

    assert_eq!(StatusCode::OK, resp.status());
    assert_eq!(
        b"timeout=5",
        resp.headers().get("Keep-Alive").unwrap().as_bytes()
    )
}

#[derive(Serialize)]
pub struct Resp {
    name: String,
}

#[handler]
async fn json() -> Json<Resp> {
    Json::new(Resp {
        name: "John".to_string(),
    })
    .header(
        HeaderName::from_str("Keep-Alive").unwrap(),
        HeaderValue::from_str("timeout=5").unwrap(),
    )
}

#[derive(Display, Debug)]
pub enum IncrementError {
    Overflow,
    Cast(TryFromIntError),
}

impl ResponderError for IncrementError {}

#[derive(Deserialize, Serialize, Debug, Path)]
pub struct Params {
    n: usize,
}

#[handler]
async fn increment_byte(#[path] p: Params) -> Result<u8, IncrementError> {
    let byte: u8 = p.n.try_into().map_err(|e| IncrementError::Cast(e))?;

    match byte {
        u8::MAX => Err(IncrementError::Overflow),
        _ => Ok(byte + 1),
    }
}

fn main() {}
