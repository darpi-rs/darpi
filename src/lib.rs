#![forbid(unsafe_code)]

pub use darpi_code_gen::{
    app, handler, job_factory, middleware, req_formatter, resp_formatter, Path, Query,
};
pub use darpi_web::{
    handler::Args, handler::Handler, job, job::RequestJobFactory, job::ResponseJobFactory, logger,
    logger::ReqFormatter, logger::RespFormatter, middleware::RequestMiddleware,
    middleware::ResponseMiddleware, request, response, xml::Xml, yaml::Yaml, Json,
};

use crate::job::Job;
pub use async_trait::async_trait;
pub use chrono;
pub use darpi_route::{ReqRoute, Route};
pub use futures;
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
use std::sync::mpsc::SendError;
pub use tokio;
use tokio::sync::oneshot;
use tokio::sync::oneshot::Receiver;

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

pub async fn oneshot<T>(job: impl Into<Job<T>>) -> Result<Receiver<T>, SendError<Job<T>>>
where
    T: Send + 'static,
{
    let job = job.into();
    match job {
        Job::Future(fut) => {
            let (otx, recv) = oneshot::channel();
            let handle = tokio::runtime::Handle::current();
            handle.spawn(async {
                let _ = otx.send(fut.into_inner().await);
            });
            Ok(recv)
        }
        Job::CpuBound(cpu) => {
            let (otx, recv) = oneshot::channel();
            rayon::spawn(move || {
                let _ = otx.send(cpu.into_inner()());
            });
            Ok(recv)
        }
        Job::IOBlocking(io_blocking) => {
            let (otx, recv) = oneshot::channel();
            let handle = tokio::runtime::Handle::current();
            handle.spawn_blocking(move || {
                let _ = otx.send(io_blocking.into_inner()());
            });
            Ok(recv)
        }
    }
}

pub async fn spawn<T>(job: impl Into<Job<T>>) -> Result<(), SendError<Job<T>>>
where
    T: Send + 'static,
{
    let job = job.into();
    match job {
        Job::Future(fut) => {
            let handle = tokio::runtime::Handle::current();
            handle.spawn(async {
                fut.into_inner().await;
            });
            Ok(())
        }
        Job::CpuBound(cpu) => {
            rayon::spawn(move || {
                cpu.into_inner()();
            });
            Ok(())
        }
        Job::IOBlocking(io_blocking) => {
            let handle = tokio::runtime::Handle::current();
            handle.spawn_blocking(move || {
                io_blocking.into_inner()();
            });
            Ok(())
        }
    }
}
