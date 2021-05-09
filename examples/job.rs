use darpi::job::{CpuJob, FutureJob, IOBlockingJob};
use darpi::{
    app, handler, job_factory, logger::DefaultFormat, middleware, App, Body, Path, Query, Request,
    RequestParts, Response,
};
use darpi_middleware::{log_request, log_response};
use env_logger;
use serde::{Deserialize, Serialize};
use std::convert::Infallible;

#[derive(Deserialize, Serialize, Debug, Query, Path)]
pub struct Name {
    name: String,
}

#[job_factory(Request)]
async fn first_async_job() -> FutureJob {
    async { println!("first job in the background.") }.into()
}

#[job_factory(Response)]
async fn first_sync_job(#[response] r: &Response<Body>) -> IOBlockingJob {
    let status_code = r.status();
    let job = move || {
        std::thread::sleep(std::time::Duration::from_secs(2));
        println!(
            "first_sync_job in the background for a request with status {}",
            status_code
        );
    };
    job.into()
}

#[job_factory(Response)]
async fn first_sync_job1() -> CpuJob {
    let job = || {
        for _ in 0..100 {
            let mut r = 0;
            for _ in 0..10000000 {
                r += 1;
            }
            println!("first_sync_job1 finished in the background. {}", r);
        }
    };
    job.into()
}

#[job_factory(Response)]
async fn first_sync_io_job() -> IOBlockingJob {
    let job = || {
        for i in 0..5 {
            std::thread::sleep(std::time::Duration::from_secs(1));
            println!("sync io finished in the background {}", i);
        }
    };
    job.into()
}

#[handler({
    jobs: {
        response: [first_sync_job, first_sync_job1]
    }
})]
async fn hello_world(#[request_parts] r: &RequestParts) -> &'static str {
    if r.headers.get("destroy-cpu-header").is_some() {
        CpuJob::from(|| {
            let mut r = 0;
            for _ in 0..10000000 {
                r += 1;
            }
            println!("first_sync_job1 finished in the background. {}", r)
        })
        .spawn()
        .expect("ohh noes");
    }

    "hello world"
}

#[handler]
async fn hello_world1() -> Result<String, String> {
    let secs = IOBlockingJob::from(move || {
        let secs = 2;
        std::thread::sleep(std::time::Duration::from_secs(secs));
        secs
    })
    .oneshot()
    .await
    .map_err(|e| format!("{}", e))?
    .await
    .map_err(|e| format!("{}", e))?;

    Ok(format!("waited {} seconds to say hello world", secs))
}

#[middleware(Request)]
pub(crate) async fn roundtrip(
    #[request] _rp: &mut Request<Body>,
    #[handler] msg: impl AsRef<str> + Send + Sync + 'static,
) -> Result<String, Infallible> {
    let res = format!("{} from roundtrip middleware", msg.as_ref());
    Ok(res)
}

#[handler({
    middleware: {
        request: [roundtrip("blah")]
    }
})]
async fn do_something(
    #[request_parts] _rp: &RequestParts,
    #[path] path: Name,
    #[middleware::request(0)] m_str: String,
) -> String {
    format!("path: {:#?} middleware: {}", path, m_str)
}

#[tokio::main]
async fn main() -> Result<(), darpi::Error> {
    env_logger::builder().is_test(true).init();

    app!({
        address: "127.0.0.1:3000",
        jobs: {
            request: [],
            response: [first_sync_io_job]
        },
        middleware: {
            request: [log_request(DefaultFormat)],
            response: [log_response(DefaultFormat, request(0))]
        },
        handlers: [{
            route: "/hello_world",
            method: GET,
            handler: hello_world
        },{
            route: "/do_something/{name}",
            method: GET,
            handler: do_something
        }]
    })
    .run()
    .await
}
