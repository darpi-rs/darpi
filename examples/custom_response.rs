use darpi::{app, handler, App, Body, Responder, Response, StatusCode};
use env_logger;

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
        }]
    })
    .run()
    .await
}
