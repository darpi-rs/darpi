#![forbid(unsafe_code)]

use async_trait::async_trait;
pub use hyper::{body::HttpBody, Body, Request, Response, StatusCode};
use job::Job;
pub use json::Json;
pub use rayon;
use std::sync::mpsc::SendError;
use tokio::sync::oneshot;
use tokio::sync::oneshot::Receiver;

pub mod handler;
pub mod job;
pub mod json;
pub mod logger;
pub mod middleware;
pub mod request;
pub mod response;
pub mod ws;
pub mod xml;
pub mod yaml;

#[async_trait]
pub trait App {
    async fn run(self) -> Result<(), hyper::Error>;
    fn shutdown_signal(&mut self) -> Option<tokio::sync::oneshot::Sender<()>>;
    fn startup_notify(&mut self) -> Option<tokio::sync::oneshot::Receiver<()>>;
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

pub fn spawn<T>(job: impl Into<Job<T>>) -> Result<(), SendError<Job<T>>>
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
