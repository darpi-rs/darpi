use darpi::{app, handler};

#[handler]
pub(crate) async fn home() -> String {
    format!("home")
}

#[darpi::main]
async fn main() -> Result<(), darpi::Error> {
    env_logger::builder().is_test(true).try_init().unwrap();

    app!({
        address: "127.0.0.1:foo",
        handlers: [{
            route: "/",
            method: GET,
            handler: home
        }]
    })
    .run()
    .await
}
