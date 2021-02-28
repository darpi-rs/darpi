use crate::{Body, Response};
use async_trait::async_trait;
use futures::Future;
use futures_util::FutureExt;
use http::request::Parts as RequestParts;
use std::pin::Pin;
use std::sync::Arc;

#[async_trait]
pub trait RequestJobFactory<C, T = ()>
where
    C: 'static + Sync + Send,
{
    type HandlerArgs;
    type Return: Into<Job<T>>;

    async fn call(
        p: &RequestParts,
        module: Arc<C>,
        b: &Body,
        ha: Self::HandlerArgs,
    ) -> Self::Return;
}

pub trait IsRequest {}

#[async_trait]
pub trait ResponseJobFactory<C, T = ()>
where
    C: 'static + Sync + Send,
{
    type HandlerArgs;
    type Return: Into<Job<T>>;

    async fn call(r: &Response<Body>, module: Arc<C>, ha: Self::HandlerArgs) -> Self::Return;
}

pub trait IsResponse {}

pub enum Job<T = ()> {
    Future(FutureJob<T>),
    CpuBound(CpuJob<T>),
    IOBlocking(IOBlockingJob<T>),
}

impl<T> From<FutureJob<T>> for Job<T> {
    fn from(fut: FutureJob<T>) -> Self {
        Self::Future(fut)
    }
}
impl<T> From<CpuJob<T>> for Job<T> {
    fn from(job: CpuJob<T>) -> Self {
        Self::CpuBound(job)
    }
}
impl<T> From<IOBlockingJob<T>> for Job<T> {
    fn from(job: IOBlockingJob<T>) -> Self {
        Self::IOBlocking(job)
    }
}

pub struct FutureJob<T = ()>(Pin<Box<dyn Future<Output = T> + Send>>);
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

impl<T> FutureJob<T> {
    pub fn into_inner(self) -> Pin<Box<dyn Future<Output = T> + Send>> {
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

impl<F, T> From<F> for FutureJob<T>
where
    F: Future<Output = T> + Send + 'static,
{
    fn from(fut: F) -> Self {
        Self(fut.boxed())
    }
}

impl<F, T> From<F> for Job<T>
where
    F: Future<Output = T> + Send + 'static,
{
    fn from(fut: F) -> Self {
        Self::Future(fut.into())
    }
}

pub fn assert_request_job(_: impl IsRequest) {}

pub fn assert_response_job(_: impl IsResponse) {}
