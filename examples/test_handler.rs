use darpi::response::ResponderError;
use darpi::{handler, Args, Body, Handler, Path, Request, StatusCode};
use derive_more::Display;
use serde::{Deserialize, Serialize};
use std::convert::TryInto;
use std::num::TryFromIntError;

#[tokio::test]
async fn increment_byte_ok() {
    use std::sync::Arc;
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
    use std::sync::Arc;
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
