use darpi::futures::{SinkExt, StreamExt};
use darpi::upgrade::Upgraded;
use darpi::{app, handler, job::FutureJob, response::UpgradeWS, App, Body, Request};
use futures::Future;
use rand::Rng;
use std::sync::Once;
use tokio::net::TcpStream;
use tokio::sync::oneshot::{Receiver, Sender};
use tokio_tungstenite::tungstenite::error::Error;
use tokio_tungstenite::{client_async, tungstenite::protocol::Message, WebSocketStream};

pub const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ\
                            abcdefghijklmnopqrstuvwxyz\
                            0123456789)(*&^%$#@!~";

#[tokio::test]
async fn websocket() {
    let (shutdown, startup, app) = make_server();
    tokio::spawn(app);
    startup.await.unwrap();

    let addr = "127.0.0.1:3000";
    let url = format!("ws://{}/websocket", addr);

    let stream = TcpStream::connect(addr).await.unwrap();

    let (mut ws_stream, _) = client_async(&url, stream).await.unwrap();

    for _ in 0..10 {
        let rand_str = make_rand_str();
        let rand_str_clone = rand_str.clone();
        ws_stream.send(rand_str.into()).await.unwrap();
        let msg = ws_stream.next().await.unwrap();

        let msg = match msg {
            Ok(m) => m,
            Err(e) => {
                panic!("client error trying to receive:  `{:#?}`", e);
            }
        };

        if msg.is_text() || msg.is_binary() {
            assert_eq!(msg.to_string(), rand_str_clone);
        }
    }

    ws_stream.close(None).await.unwrap();
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

async fn handle_connection(stream: Upgraded) {
    let mut ws_stream = WebSocketStream::from_raw_socket(
        stream,
        tokio_tungstenite::tungstenite::protocol::Role::Server,
        None,
    )
    .await;

    while let Some(item) = ws_stream.next().await {
        match item {
            Ok(msg) => match msg {
                Message::Text(text) => {
                    ws_stream.send(Message::Text(text)).await.unwrap();
                }
                Message::Close(_) => {
                    if let Err(e) = ws_stream.close(None).await {
                        match e {
                            Error::ConnectionClosed => (),
                            _ => {
                                println!("Error while closing: {:#?}", e);
                                break;
                            }
                        }
                    }
                    break;
                }
                _ => (),
            },
            Err(e) => println!("server error {:?}", e),
        }
    }
}

#[handler]
async fn web_socket_handler(#[request] r: Request<Body>) -> Result<UpgradeWS, String> {
    let resp = UpgradeWS::from_header(r.headers())
        .ok_or("missing SEC_WEBSOCKET_KEY header".to_string())?;

    FutureJob::from(async move {
        let upgraded = darpi::upgrade::on(r).await.unwrap();
        handle_connection(upgraded).await;
    })
    .spawn()
    .map_err(|e| format!("{}", e))?;

    Ok(resp)
}

static ONCE: Once = Once::new();

pub fn make_server() -> (Sender<()>, Receiver<()>, impl Future<Output = ()>) {
    ONCE.call_once(|| {
        //std::env::set_var("RUST_LOG", "debug");
        env_logger::builder().is_test(true).init();
    });

    let mut app = app!({
        address: "127.0.0.1:3000",
        handlers: [{
            route: "/websocket",
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
