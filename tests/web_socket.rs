use darpi::futures::{SinkExt, StreamExt};
use darpi::{app, handler, job::FutureJob, response::UpgradeWS, Body, Request};
use futures::Future;
use http::header::{CONNECTION, SEC_WEBSOCKET_KEY, UPGRADE};
use http::StatusCode;
use rand::Rng;
use std::sync::Once;
use tokio::sync::oneshot::{Receiver, Sender};
use tokio_tungstenite::{
    connect_async, tungstenite::protocol::Message, tungstenite::protocol::Role, WebSocketStream,
};

mod common;

use common::convert_key;
use reqwest::Response;
use tokio_tungstenite::tungstenite::error::Error;

pub const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ\
                            abcdefghijklmnopqrstuvwxyz\
                            0123456789)(*&^%$#@!~";

async fn make_req() -> Response {
    let client = reqwest::Client::new();

    let req = client
        .get("http://127.0.0.1:3000")
        .header(CONNECTION, "upgrade")
        .header(UPGRADE, "websocket")
        .header(SEC_WEBSOCKET_KEY, convert_key("password".as_bytes()))
        .build()
        .unwrap();

    let resp = client.execute(req).await.unwrap();
    assert_eq!(StatusCode::SWITCHING_PROTOCOLS, resp.status());
    assert_eq!(Some(0), resp.content_length());
    assert_eq!("websocket", resp.headers().get(UPGRADE).unwrap());
    assert_eq!("upgrade", resp.headers().get(CONNECTION).unwrap());

    resp
}

#[tokio::test]
async fn websocket() {
    let (shutdown, startup, app) = make_server();
    tokio::spawn(app);
    startup.await.unwrap();

    make_req().await;

    let (mut ws_stream, _) = connect_async("ws://127.0.0.1:3000")
        .await
        .expect("Failed to connect");

    let mut msgs = vec![];
    for _ in 0..10 {
        let rand_str_clone = make_rand_str();
        msgs.push(rand_str_clone);
    }

    let (rx, mut tx) = tokio::sync::mpsc::channel(10);

    let msg_clone = msgs.clone();
    let join = tokio::spawn(async move {
        let mut i = 0usize;
        while let Some(msg) = tx.recv().await {
            ws_stream.send(msg).await.unwrap();
            let msg = ws_stream.next().await.unwrap();

            let msg = match msg {
                Ok(m) => m,
                Err(e) => {
                    panic!("client error trying to receive:  `{:#?}`", e);
                }
            };

            if msg.is_text() || msg.is_binary() {
                assert_eq!(msg.to_string(), msg_clone[i]);
            }
            i += 1;
        }
        assert_eq!(i, msg_clone.len());

        ws_stream.close(None).await.unwrap();
        SinkExt::close(&mut ws_stream).await.unwrap();
        println!("closing client websocket");
    });

    for msg in msgs {
        rx.send(Message::from(msg)).await.unwrap();
    }
    drop(rx);
    join.await.unwrap();
    shutdown.send(()).unwrap();
}

fn make_rand_str() -> String {
    let mut rng = rand::thread_rng();
    let n: usize = rng.gen_range(1..1024);
    let mut rand_str = String::with_capacity(n);

    for _ in 0..n {
        let idx = rng.gen_range(0..CHARSET.len());
        rand_str.push(CHARSET[idx] as char);
    }
    rand_str
}

#[handler]
async fn web_socket_handler(#[request] r: Request<Body>) -> Result<UpgradeWS, String> {
    let resp = UpgradeWS::from_header(r.headers())
        .ok_or("missing SEC_WEBSOCKET_KEY header".to_string())?;

    FutureJob::from(async move {
        let upgraded = darpi::upgrade::on(r).await.unwrap();
        let mut ws_stream = WebSocketStream::from_raw_socket(upgraded, Role::Server, None).await;

        while let Some(msg) = ws_stream.next().await {
            let msg = match msg {
                Ok(m) => m,
                Err(e) => {
                    match e {
                        Error::ConnectionClosed | Error::Protocol(_) | Error::Utf8 => (),
                        err => panic!("Testcase failed: {:#?}", err),
                    }
                    return;
                }
            };

            if msg.is_text() || msg.is_binary() {
                if let Err(e) = ws_stream.send(msg).await {
                    panic!("error trying to send:  `{:#?}`", e);
                }
            } else if msg.is_close() {
                ws_stream.close(None).await.unwrap();
                SinkExt::close(&mut ws_stream).await.unwrap();
                println!("closing server websocket");
                return;
            }
        }
    })
    .spawn()
    .map_err(|e| format!("{}", e))?;

    Ok(resp)
}

static ONCE: Once = Once::new();

pub fn make_server() -> (Sender<()>, Receiver<()>, impl Future<Output = ()>) {
    ONCE.call_once(|| {
        env_logger::builder().is_test(true).init();
    });

    let mut app = app!({
        address: "127.0.0.1:3000",
        handlers: [{
            route: "/",
            method: GET,
            handler: web_socket_handler
        }]
    });

    let shutdown = app.shutdown_signal().unwrap();
    let startup = app.startup_notify().unwrap();

    (shutdown, startup, async {
        app.run().await.unwrap();
    })
}
