use chrono::{DateTime, Utc};
use http::header::FORWARDED;
pub use hyper::{body::HttpBody, Body, Request, Response, StatusCode};
use std::time::Instant;

pub struct DefaultFormat;
impl ReqFormatter for DefaultFormat {}
impl RespFormatter for DefaultFormat {}

pub trait ReqFormatter: Sync + Send + 'static {
    fn format_req(&self, r: &Request<Body>) -> String {
        let mut content = vec!["[darpi::request]".to_string()];

        if let Some(forwarded) = r.headers().get(FORWARDED) {
            let forwarded = format!(
                "remote_ip: [{}]",
                forwarded.to_str().map_err(|_| "").expect("never to happen")
            );
            content.push(forwarded);
        }

        let now: DateTime<Utc> = Utc::now();
        let now = format!("when: [{}]", now.to_rfc3339());
        content.push(now);

        let uri = format!("uri: [{:#?}]", r.uri());
        content.push(uri);

        let body_size = match r.size_hint().exact() {
            Some(s) => s,
            None => r.size_hint().lower(),
        };

        let size = format!("body_size: [{}] bytes", body_size);
        content.push(size);

        content.join(" ").into()
    }
}

pub trait RespFormatter: Sync + Send + 'static {
    fn format_resp(&self, start: &Instant, r: &Response<Body>) -> String {
        let mut content = vec!["[darpi::response]".to_string()];

        if let Some(forwarded) = r.headers().get(FORWARDED) {
            let forwarded = format!(
                "remote_ip: [{}]",
                forwarded.to_str().map_err(|_| "").expect("never to happen")
            );
            content.push(forwarded);
        }

        let now: DateTime<Utc> = Utc::now();
        let now = format!("when: [{}]", now.to_rfc3339());
        content.push(now);

        let took = format!("took: {:#?}", start.elapsed());

        content.push(took);

        let body_size = match r.size_hint().exact() {
            Some(s) => s,
            None => r.size_hint().lower(),
        };

        let size = format!("body_size: [{}] bytes", body_size);
        content.push(size);

        content.join(" ").into()
    }
}
