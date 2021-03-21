use darpi::futures::{SinkExt, StreamExt};
use darpi::{
    app, handler, job::FutureJob, response::UpgradeWS, Body, Method, Request, RequestParts,
    Response, StatusCode,
};
use shaku::module;
use tokio_tungstenite::{tungstenite::protocol::Role, WebSocketStream};

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

#[handler]
async fn hello_world(#[request] r: Request<Body>) -> Result<UpgradeWS, String> {
    let resp = UpgradeWS::from_header(r.headers());

    darpi::spawn(FutureJob::from(async move {
        let upgraded = darpi::upgrade::on(r).await.unwrap();
        let mut ws_stream = WebSocketStream::from_raw_socket(upgraded, Role::Server, None).await;

        while let Some(msg) = ws_stream.next().await {
            let msg = msg.unwrap();

            if msg.is_text() || msg.is_binary() {
                println!("received a message `{}`", msg);
                ws_stream.send(msg).await.unwrap();
            } else if msg.is_close() {
                println!("closing websocket");
                return;
            }
        }
    }))
    .await
    .map_err(|e| format!("{}", e))?;

    Ok(resp.unwrap())
}

//todo fix when container missing

//RUST_LOG=darpi=info cargo test --test job -- --nocapture
//#[tokio::test]
#[tokio::main]
async fn main() -> Result<(), darpi::Error> {
    app!({
        address: "127.0.0.1:3000",
        container: {
            factory: make_container(),
            type: Container
        },
        handlers: [{
            route: "/",
            method: Method::GET,
            handler: hello_world
        }]
    })
    .run()
    .await
}
