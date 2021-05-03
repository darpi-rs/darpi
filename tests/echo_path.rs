use darpi::header::CONTENT_TYPE;
use darpi::{app, handler, Path, Query, StatusCode};
use futures::Future;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::sync::Once;
use tokio::sync::oneshot::{Receiver, Sender};

#[derive(Deserialize, Serialize, Debug, Query, Path)]
pub struct Input {
    pub echo: String,
}

pub const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ\
                            abcdefghijklmnopqrstuvwxyz\
                            0123456789)(*&^%$@!~";

#[handler]
async fn echo_path_handler(#[path] q: Input) -> String {
    q.echo
}

#[tokio::test]
async fn echo_path() {
    let (shutdown, startup, app) = make_server();
    tokio::spawn(app);
    startup.await.unwrap();

    let mut rng = rand::thread_rng();
    let n: usize = rng.gen_range(1..1024);
    let mut rand_str = String::with_capacity(n);

    for _ in 0..n {
        let idx = rng.gen_range(0..CHARSET.len());
        rand_str.push(CHARSET[idx] as char);
    }

    let rand_str_clone = rand_str.clone();

    let client = reqwest::Client::new();

    let req = client
        .get(format!("http://127.0.0.1:3000/echopath/{}", rand_str))
        .build()
        .unwrap();

    let resp = client.execute(req).await.unwrap();

    assert_eq!(StatusCode::OK, resp.status());
    assert_eq!(
        "text/plain; charset=utf-8",
        resp.headers().get(CONTENT_TYPE).unwrap()
    );
    assert_eq!(rand_str_clone, resp.text().await.unwrap());
    shutdown.send(()).unwrap();
}

static ONCE: Once = Once::new();

pub fn make_server() -> (Sender<()>, Receiver<()>, impl Future<Output = ()>) {
    ONCE.call_once(|| {
        env_logger::builder().is_test(true).init();
    });

    let mut app = app!({
        address: "127.0.0.1:3000",
        handlers: [{
            route: "/echopath/{echo}",
            method: GET,
            handler: echo_path_handler
        }]
    });

    let shutdown = app.shutdown_signal().unwrap();
    let startup = app.startup_notify().unwrap();

    (shutdown, startup, async {
        app.run().await.unwrap();
    })
}
