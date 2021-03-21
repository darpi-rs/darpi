#![feature(type_alias_impl_trait)]
#![feature(associated_type_defaults)]
#![feature(min_type_alias_impl_trait)]

use crate::{Body, Request, Response};
use async_trait::async_trait;
use futures::Future;
use futures_util::FutureExt;
use std::pin::Pin;
use std::sync::mpsc::SendError;
use std::sync::Arc;
use tokio::sync::oneshot;
use tokio::sync::oneshot::Receiver;

#[async_trait]
pub trait RequestJobFactory<C, T = ()>
where
    C: 'static + Sync + Send,
{
    type HandlerArgs;
    type Return: Job<T>;

    async fn call(p: &Request<Body>, module: Arc<C>, ha: Self::HandlerArgs) -> Self::Return;
}

pub trait IsRequest {}

#[async_trait]
pub trait ResponseJobFactory<C, T = ()>
where
    C: 'static + Sync + Send,
{
    type HandlerArgs;
    type Return: Job<T>;

    async fn call(r: &Response<Body>, module: Arc<C>, ha: Self::HandlerArgs) -> Self::Return;
}

pub trait IsResponse {}

#[async_trait]
pub trait Job<T = ()>: Sized {
    async fn oneshot(self) -> Result<Receiver<T>, SendError<Self>>;
    async fn spawn(self) -> Result<(), SendError<Self>>;
}

pub trait InnerJob<T>: Sized {
    type Type = impl Future<Output = T> + Send;
}

#[async_trait]
impl<T, F> InnerJob<T> for F
where
    T: 'static + Send + Sync,
    F: Future<Output = T> + Send + 'static,
{
}

#[async_trait]
impl<T, F> Job<T> for F
where
    T: 'static + Send + Sync,
    F: Future<Output = T> + Send + 'static,
{
    async fn oneshot(self) -> Result<Receiver<T>, SendError<Self>> {
        let (otx, recv) = oneshot::channel();
        let handle = tokio::runtime::Handle::current();
        handle.spawn(async {
            let _ = otx.send(self.await);
        });
        Ok(recv)
    }

    async fn spawn(self) -> Result<(), SendError<Self>> {
        let handle = tokio::runtime::Handle::current();
        handle.spawn(self);
        Ok(())
    }
}

pub struct FutureJob<F, T = ()>(F)
where
    F: FnOnce() -> T + Send;
pub struct CpuJob<T = ()>(Box<dyn FnOnce() -> T + Send>);
pub struct IOBlockingJob<T = ()>(Box<dyn FnOnce() -> T + Send>);

impl<T> IOBlockingJob<T> {
    pub fn into_inner(self) -> Box<dyn FnOnce() -> T + Send> {
        self.0
    }
}

impl<T> CpuJob<T> {
    pub fn into_inner(self) -> Box<dyn FnOnce() -> T + Send> {
        self.0
    }
}

impl<F, T> From<F> for IOBlockingJob<T>
where
    F: FnOnce() -> T + Send + 'static,
{
    fn from(func: F) -> Self {
        Self(Box::new(func))
    }
}

impl<F, T> From<F> for CpuJob<T>
where
    F: FnOnce() -> T + Send + 'static,
{
    fn from(func: F) -> Self {
        Self(Box::new(func))
    }
}

pub fn assert_request_job(_: impl IsRequest) {}

pub fn assert_response_job(_: impl IsResponse) {}
