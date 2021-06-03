use darpi::futures::{SinkExt, StreamExt};
use darpi::{app, handler, job::FutureJob, response::UpgradeWS, App, Body, Request};
use tokio_tungstenite::{tungstenite::protocol::Role, WebSocketStream};

#[handler]
async fn hello_world(#[request] r: Request<Body>) -> Result<UpgradeWS, String> {
    let resp = UpgradeWS::from_header(r.headers())
        .ok_or("missing SEC_WEBSOCKET_KEY header".to_string())?;

    FutureJob::from(async move {
        let upgraded = darpi::upgrade::on(r).await.unwrap();
        let mut ws_stream = WebSocketStream::from_raw_socket(upgraded, Role::Server, None).await;

        while let Some(msg) = ws_stream.next().await {
            let msg = match msg {
                Ok(m) => m,
                Err(e) => {
                    println!("error trying to receive:  `{:#?}`", e);
                    return;
                }
            };

            if msg.is_text() || msg.is_binary() {
                println!("received a message `{}`", msg);
                if let Err(e) = ws_stream.send(msg).await {
                    println!("error trying to send:  `{:#?}`", e);
                    return;
                }
            } else if msg.is_close() {
                println!("closing websocket");
                return;
            }
        }
    })
    .spawn()
    .map_err(|e| format!("{}", e))?;

    Ok(resp)
}

#[darpi::main]
async fn main() -> Result<(), darpi::Error> {
    app!({
        address: "127.0.0.1:3000",
        handlers: [{
            route: "/",
            method: GET,
            handler: hello_world
        }]
    })
    .run()
    .await
}
