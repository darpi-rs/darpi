#![forbid(unsafe_code)]

pub use darpi_code_gen::{
    app, handler, job_factory, main, middleware, req_formatter, resp_formatter, test, Path, Query,
};
pub use darpi_web::{
    handler::Args, handler::Handler, job, job::RequestJobFactory, job::ResponseJobFactory, logger,
    logger::ReqFormatter, logger::RespFormatter, middleware::RequestMiddleware,
    middleware::ResponseMiddleware, oneshot, request, response, spawn, xml::Xml, yaml::Yaml, App,
    Json,
};

pub trait Route<T = ()> {
    fn is_match(method: &str) -> bool;
    fn get_tuple_args(route_str: &str, req: &Vec<(usize, usize)>) -> T;
}

pub use async_trait::async_trait;
pub use chrono;
pub use futures;
pub use gonzales;
pub use http::{header, request::Parts as RequestParts, Method, StatusCode};
pub use hyper::upgrade;
pub use hyper::{self, body, body::HttpBody, service, Body, Error, Request, Response, Server};
pub use log;
pub use rayon;
use serde::{de, Deserialize, Deserializer};
pub use serde_json;
pub use shaku;
use shaku::module;
use std::fmt::Display;
use std::str::FromStr;
pub use tokio;

module! {
    pub EmptyContainer {
        components = [],
        providers = [],
    }
}

pub fn make_empty_container() -> EmptyContainer {
    EmptyContainer::builder().build()
}

pub fn from_str<'de, T, D>(deserializer: D) -> Result<T, D::Error>
where
    T: FromStr,
    T::Err: Display,
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    T::from_str(&s).map_err(de::Error::custom)
}
