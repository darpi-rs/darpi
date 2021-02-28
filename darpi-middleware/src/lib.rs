use darpi::{
    logger::ReqFormatter, logger::RespFormatter, middleware, request::PayloadError, Body, HttpBody,
    RequestParts, Response,
};

pub mod auth;
pub mod compression;

use log;
use std::convert::Infallible;
use std::time::Instant;

/// this middleware limits the request body size by a user passed argument
/// the argument `size` indicates number of bytes
/// if the body is higher than the specified size, it will result in an error response being sent to the user
/// ```rust
/// #[handler([body_size_limit(64)])]
/// async fn say_hello(#[path] p: Name, #[body] payload: Json<Name>) -> impl Responder {
///     format!("{} sends hello to {}", p.name, payload.name)
/// }
/// ```
#[middleware(Request)]
pub async fn body_size_limit(#[body] b: &Body, #[handler] size: u64) -> Result<(), PayloadError> {
    if let Some(limit) = b.size_hint().upper() {
        if size < limit {
            return Err(PayloadError::Size(size, limit));
        }
    }
    Ok(())
}

#[middleware(Request)]
pub async fn log_request(
    #[request_parts] rp: &RequestParts,
    #[body] b: &Body,
    #[handler] formatter: impl ReqFormatter,
) -> Result<Instant, Infallible> {
    let formatted = formatter.format_req(b, rp);
    log::info!("{}", formatted);
    Ok(Instant::now())
}

#[middleware(Response)]
pub async fn log_response(
    #[response] r: &Response<Body>,
    #[handler] formatter: impl RespFormatter,
    #[handler] start: Instant,
) {
    let formatted = formatter.format_resp(&start, r);
    log::info!("{}", formatted);
}
