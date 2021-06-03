use criterion::{criterion_group, criterion_main, Criterion};
use darpi::hyper::service::{make_service_fn, service_fn};
use darpi::hyper::{Body, Request, Response, Server};
use darpi::tokio::runtime::Runtime;
use darpi::tokio::sync::oneshot::Sender;
use rand::Rng;
use serde_urlencoded;
use std::convert::Infallible;

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

fn echo_hyper(c: &mut Criterion) {
    let (shutdown, runtime) = make_server();

    let client = reqwest::Client::new();
    c.bench_function("echo hyper", |b| {
        b.to_async(&runtime).iter(|| {
            let rand_str = get_random_str();

            let req = client
                .get("http://127.0.0.1:3000/echo")
                .query(&[&("echo", rand_str)])
                .build()
                .unwrap();
            async {
                let _ = client.execute(req).await.unwrap();
            }
        });
    });
    shutdown.send(()).unwrap();
}

async fn hello(r: Request<Body>) -> Result<Response<Body>, Infallible> {
    let (parts, _) = r.into_parts();
    let query_str = parts.uri.query().unwrap().to_string();

    let query = serde_urlencoded::from_str::<Vec<(String, String)>>(&query_str).unwrap();
    for (name, value) in query {
        if name == "echo" {
            return Ok(Response::new(Body::from(value)));
        }
    }
    Ok(Response::new(Body::empty()))
}

fn make_server() -> (Sender<()>, Runtime) {
    let make_svc = make_service_fn(|_conn| async { Ok::<_, Infallible>(service_fn(hello)) });

    let addr = ([127, 0, 0, 1], 3000).into();

    let runtime = tokio::runtime::Runtime::new().unwrap();
    let server = runtime
        .block_on(async { Server::bind(&addr) })
        .serve(make_svc);

    let (tx, rx) = tokio::sync::oneshot::channel::<()>();
    let graceful = server.with_graceful_shutdown(async {
        rx.await.ok();
    });

    runtime.spawn(graceful);
    (tx, runtime)
}

criterion_group!(benches, echo_hyper);
criterion_main!(benches);
