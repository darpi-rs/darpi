use darpi::{app, handler, job::IOBlockingJob, job_factory, App, RequestParts};

#[job_factory(Response)]
async fn sleep_blocking() -> IOBlockingJob {
    let job = || {
        for _ in 0..5 {
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
    };
    job.into()
}

#[handler({
    jobs: {
        response: [sleep_blocking]
    }
})]
async fn hello_world(#[request_parts] _: &RequestParts) -> &'static str {
    "hello world"
}

#[darpi::main]
async fn main() -> Result<(), darpi::Error> {
    app!({
        address: "127.0.0.1:3000",
        handlers: [{
            route: "/hello_world",
            method: POST,
            handler: hello_world
        }]
    })
    .run()
    .await
}
