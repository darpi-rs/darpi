use criterion::{criterion_group, criterion_main, Criterion};
use darpi::tokio::runtime::Runtime;
use darpi::{app, handler, App};
use env_logger;
use futures::Future;
use tokio::sync::oneshot::{Receiver, Sender};

fn hello_world(c: &mut Criterion) {
    let (shutdown, runtime) = setup();

    c.bench_function("hello world", |b| {
        b.to_async(&runtime)
            .iter(|| reqwest::get("http://127.0.0.1:3000/hello_world"));
    });
    shutdown.send(()).unwrap();
}

#[handler]
async fn hello_world_handler() -> &'static str {
    "hello world"
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

fn setup() -> (Sender<()>, Runtime) {
    let (shutdown, startup, app) = make_server();
    let runtime = tokio::runtime::Runtime::new().unwrap();
    runtime.spawn(app);
    runtime.block_on(startup).unwrap();
    (shutdown, runtime)
}

criterion_group!(benches, hello_world);
criterion_main!(benches);
