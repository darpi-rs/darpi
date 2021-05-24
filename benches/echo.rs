use criterion::{criterion_group, criterion_main, Criterion};
use darpi::tokio::runtime::Runtime;
use darpi::{app, handler, App, Path, Query};
use env_logger;
use futures::Future;
use serde::{Deserialize, Serialize};
use tokio::sync::oneshot::{Receiver, Sender};

#[derive(Deserialize, Serialize, Debug, Query, Path)]
pub struct Input {
    echo: String,
}

use rand::Rng;
use std::sync::Once;

const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ\
                            abcdefghijklmnopqrstuvwxyz\
                            0123456789)(*&^%$#@!~";

fn get_random_str() -> String {
    let mut rng = rand::thread_rng();
    let n: usize = rng.gen_range(1..1024);
    let mut rand_str = String::with_capacity(n);

    for _ in 0..n {
        let idx = rng.gen_range(0..CHARSET.len());
        rand_str.push(CHARSET[idx] as char);
    }
    rand_str
}

fn echo_path(c: &mut Criterion) {
    let (shutdown, runtime) = setup();
    let client = reqwest::Client::new();

    c.bench_function("echo", |b| {
        b.to_async(&runtime).iter(|| {
            let rand_str = get_random_str();
            let req = client
                .get(format!("http://127.0.0.1:3000/echo/{}", rand_str))
                .build()
                .unwrap();
            async {
                let _ = client.execute(req).await.unwrap();
            }
        });
    });
    shutdown.send(()).unwrap();
}

fn echo(c: &mut Criterion) {
    let (shutdown, runtime) = setup();

    let client = reqwest::Client::new();
    c.bench_function("echo", |b| {
        b.to_async(&runtime).iter(|| {
            let rand_str = get_random_str();
            let i = Input { echo: rand_str };

            let req = client
                .get("http://127.0.0.1:3000/echo")
                .query(&i)
                .build()
                .unwrap();
            async {
                let _ = client.execute(req).await.unwrap();
            }
        });
    });
    shutdown.send(()).unwrap();
}

#[handler]
async fn echo_query_handler(#[query] q: Input) -> String {
    q.echo
}

#[handler]
async fn echo_path_handler(#[path] q: Input) -> String {
    q.echo
}

static ONCE: Once = Once::new();

fn make_server() -> (Sender<()>, Receiver<()>, impl Future<Output = ()>) {
    ONCE.call_once(|| {
        env_logger::builder().is_test(true).init();
    });

    let mut app = app!({
        address: "127.0.0.1:3000",
        handlers: [{
            route: "/echo",
            method: GET,
            handler: echo_query_handler
        },{
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

fn setup() -> (Sender<()>, Runtime) {
    let (shutdown, startup, app) = make_server();
    let runtime = tokio::runtime::Runtime::new().unwrap();

    runtime.spawn(app);
    runtime.block_on(startup).unwrap();
    (shutdown, runtime)
}

criterion_group!(benches, echo, echo_path);
criterion_main!(benches);
