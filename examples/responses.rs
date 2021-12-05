use darpi::{app, handler, App, Body, Json, Responder, Response, StatusCode};
use env_logger;
use http::header::HeaderName;
use http::HeaderValue;
use serde::Serialize;
use std::str::FromStr;

pub struct HelloWorldResp;

impl Responder for HelloWorldResp {
    fn respond(self) -> Response<Body> {
        Response::builder()
            .header("my_custom_header", "application/json")
            .status(StatusCode::ACCEPTED)
            .body(Body::empty())
            .expect("this cannot happen")
    }
}

#[handler]
async fn hello_world() -> HelloWorldResp {
    HelloWorldResp
}

#[handler]
async fn hello_world1() -> Response<Body> {
    Response::builder()
        .header("my_custom_header", "application/json")
        .status(StatusCode::ACCEPTED)
        .body(Body::empty())
        .expect("this cannot happen")
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

#[darpi::main]
async fn main() -> Result<(), darpi::Error> {
    env_logger::builder().is_test(true).init();

    app!({
        address: "127.0.0.1:3000",
        handlers: [{
            route: "/hello_world",
            method: GET,
            handler: hello_world
        },{
            route: "/hello_world1",
            method: GET,
            handler: hello_world1
        },{
            route: "/json",
            method: GET,
            handler: json
        }]
    })
    .run()
    .await
}
