use darpi::futures::{SinkExt, StreamExt};
use darpi::{
    app, handler, job::FutureJob, response::SwitchingProtocol, Method, Path, Query, Request,
    RequestParts, Response, StatusCode,
};
use hyper::upgrade::Upgraded;
use serde::Serialize;
use shaku::module;
use tokio_tungstenite::{accept_async, tungstenite::Message};

fn make_container() -> Container {
    let module = Container::builder().build();
    module
}

module! {
    Container {
        components = [],
        providers = [],
    }
}

//todo disallow body extractors when having ws
#[handler]
async fn hello_world(#[ws] upgraded: Upgraded) -> Result<SwitchingProtocol, String> {
    let mut ws_stream = accept_async(upgraded).await.expect("Failed to accept");
    darpi::spawn(FutureJob::from(async move {
        while let Some(msg) = ws_stream.next().await {
            let msg = msg.unwrap();

            match msg {
                Message::Close(c) => {
                    ws_stream.close(c).await.unwrap();
                    return;
                }
                _ => {}
            }
            if msg.is_text() || msg.is_binary() {
                ws_stream.send(msg).await.unwrap();
            }
        }
    }))
    .await
    .map_err(|e| format!("{}", e))?;
    Ok(SwitchingProtocol)
}

//todo fix when container missing

//RUST_LOG=darpi=info cargo test --test job -- --nocapture
//#[tokio::test]
#[tokio::test]
async fn main() -> Result<(), darpi::Error> {
    app!({
        address: "127.0.0.1:3000",
        container: {
            factory: make_container(),
            type: Container
        },
        handlers: [{
            route: "/hello_world",
            method: Method::GET,
            handler: hello_world
        }]
    })
    .run()
    .await
}
