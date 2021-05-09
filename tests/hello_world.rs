use darpi::header::CONTENT_TYPE;
use darpi::{app, handler, App, StatusCode};
use env_logger;
use futures::Future;
use tokio::sync::oneshot::{Receiver, Sender};

#[handler]
async fn hello_world_handler() -> &'static str {
    "hello world"
}

#[tokio::test]
async fn hello_world() {
    let (shutdown, startup, app) = make_server();
    tokio::spawn(app);
    startup.await.unwrap();

    let resp = reqwest::get("http://127.0.0.1:3000/hello_world")
        .await
        .unwrap();

    assert_eq!(StatusCode::OK, resp.status());
    assert_eq!(
        "text/plain; charset=utf-8",
        resp.headers().get(CONTENT_TYPE).unwrap()
    );
    assert_eq!("hello world", resp.text().await.unwrap());
    shutdown.send(()).unwrap();
}

fn make_server() -> (Sender<()>, Receiver<()>, impl Future<Output = ()>) {
    env_logger::builder().is_test(true).init();

    let mut app = app!({
        address: "127.0.0.1:3000",
        handlers: [{
            route: "/hello_world",
            method: GET,
            handler: hello_world_handler
        }]
    });

    let shutdown = app.shutdown_signal().unwrap();
    let startup = app.startup_notify().unwrap();

    (shutdown, startup, async {
        app.run().await.unwrap();
    })
}
