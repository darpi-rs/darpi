use darpi::futures::{SinkExt, StreamExt};
use darpi::header::HeaderMap;
use darpi::hyper::Uri;
use darpi::{
    app, handler, job::FutureJob, response::UpgradeWS, Method, Request, RequestParts, Response,
    StatusCode,
};
use darpi_middleware::request_parts;
use hyper::upgrade::Upgraded;
use shaku::module;
use tokio_tungstenite::accept_async;

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
#[handler({
    middleware: {
        request: [request_parts]
    }
})]
async fn hello_world(
    #[middleware::request(0)] _parts: (HeaderMap, Uri),
    #[ws] upgraded: Upgraded,
) -> Result<UpgradeWS, String> {
    let mut ws_stream = accept_async(upgraded).await.expect("Failed to accept");

    //this runs in the background
    darpi::spawn(FutureJob::from(async move {
        while let Some(msg) = ws_stream.next().await {
            let msg = msg.unwrap();

            if msg.is_text() || msg.is_binary() {
                ws_stream.send(msg).await.unwrap();
            } else if msg.is_close() {
                return;
            }
        }
    }))
    .await
    .map_err(|e| format!("{}", e))?;

    Ok(UpgradeWS)
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
