use crate::body::{BoxBody, LocalBoxBody};
use http::{Request, Response};
use crate::transport::BoxFuture;
use crate::codegen::LocalBoxFuture;
use std::convert::Infallible;
use std::{future::Future, sync::Arc};

use bytes::Bytes;
use hyper::Body;
pub(crate) use hyper::rt::Executor;
use tower_service::Service;

pub trait BoxCloneService: Service<Request<Body>, Response = Response<Self::BoxBody>, Future = Self::BoxFuture> + Clone {
    type BoxBody;
    type BoxFuture: Future<Output = Result<Response<Self::BoxBody>, Infallible>>;

    fn empty_response() -> Self::BoxFuture;
}

impl BoxCloneService for tower::util::BoxCloneService<Request<Body>, Response<BoxBody>, Infallible> {
    type BoxBody = BoxBody;
    type BoxFuture = BoxFuture<'static, Result<Response<Self::BoxBody>, Infallible>>;

    fn empty_response() -> Self::BoxFuture {
        Box::pin(
            async move {
                Ok(
                    Response::builder()
                        .status(http::StatusCode::OK)
                        .header("grpc-status", "12")
                        .header("content-type", "application/grpc")
                        .body(crate::body::empty_body())
                        .unwrap()
                )
        })
    }
}

impl BoxCloneService for crate::util::LocalBoxCloneService<Request<Body>, Response<LocalBoxBody>, Infallible> {
    type BoxBody = LocalBoxBody;
    type BoxFuture = LocalBoxFuture<Response<Self::BoxBody>, Infallible>;

    fn empty_response() -> Self::BoxFuture {
        Box::pin(
            async move {
                Ok(
                    Response::builder()
                        .status(http::StatusCode::OK)
                        .header("grpc-status", "12")
                        .header("content-type", "application/grpc")
                        .body(crate::body::local_empty_body())
                        .unwrap()
                )
        })
    }
}

pub trait ResBodyExecutor<ResBody> {

}

impl<ResBody> ResBodyExecutor<ResBody> for TokioExec
where
    ResBody: http_body::Body<Data = Bytes> + Send + 'static,
    ResBody::Error: Into<crate::Error>,
{

}

impl<ResBody> ResBodyExecutor<ResBody> for LocalExec
where
    ResBody: http_body::Body<Data = Bytes> + 'static,
    ResBody::Error: Into<crate::Error>,
{

}

pub trait HasBoxCloneService {
    type BoxCloneService: BoxCloneService;
}

impl HasBoxCloneService for TokioExec {
    type BoxCloneService = tower::util::BoxCloneService<Request<Body>, Response<BoxBody>, Infallible>;
}

impl HasBoxCloneService for LocalExec {
    type BoxCloneService = crate::util::LocalBoxCloneService<Request<Body>, Response<LocalBoxBody>, Infallible>;
}

pub trait MakeBoxCloneService<S>: HasBoxCloneService {
    fn box_clone_service(svc: S) -> Self::BoxCloneService;
}

impl<S> MakeBoxCloneService<S> for TokioExec
where
    S: Service<Request<Body>, Response = Response<BoxBody>, Error = Infallible> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    fn box_clone_service(svc: S) -> Self::BoxCloneService {
        Self::BoxCloneService::new(svc)
    }
}

impl<S> MakeBoxCloneService<S> for LocalExec
where
    S: Service<Request<Body>, Response = Response<LocalBoxBody>, Error = Infallible> + Clone + 'static,
    S::Future: 'static,
{
    fn box_clone_service(svc: S) -> Self::BoxCloneService {
        Self::BoxCloneService::new(svc)
    }
}

pub trait HttpServiceExecutor<S, ResBody>: HasBoxCloneService + ResBodyExecutor<ResBody> + FutureExecutor<S::Future, ResBody>
where
    S: Service<Request<Body>, Response = Response<ResBody>>,
{
    type BoxService;
}

impl<S, ResBody> HttpServiceExecutor<S, ResBody> for TokioExec
where
    Self: ResBodyExecutor<ResBody>,
    S: Service<Request<Body>, Response = Response<ResBody>> + Clone + Send + 'static,
    S::Future: Send + 'static,
    S::Error: Into<crate::Error> + Send,
    ResBody: http_body::Body<Data = Bytes> + Send + 'static,
    ResBody::Error: Into<crate::Error>,
{
    type BoxService = tower::util::UnsyncBoxService<Request<Body>, Response<Self::BoxHttpBody>, crate::Error>;
}

impl<S, ResBody> HttpServiceExecutor<S, ResBody> for LocalExec
where
    Self: ResBodyExecutor<ResBody>,
    S: Service<Request<Body>, Response = Response<ResBody>> + Clone + 'static,
    S::Future: 'static,
    S::Error: Into<crate::Error> + Send,
    ResBody: http_body::Body<Data = Bytes> + 'static,
    ResBody::Error: Into<crate::Error>,
{
    type BoxService = tower::util::UnsyncBoxService<Request<Body>, Response<Self::BoxHttpBody>, crate::Error>;
}

pub trait FutureExecutor<F, ResBody>: ResBodyExecutor<ResBody> {
    type BoxHttpBody;

    fn box_http_body(body: ResBody) -> Self::BoxHttpBody;
}

impl<F, ResBody> FutureExecutor<F, ResBody> for TokioExec
where
    Self: ResBodyExecutor<ResBody>,
    ResBody: http_body::Body<Data = Bytes> + Send + 'static,
    ResBody::Error: Into<crate::Error>,
{
    type BoxHttpBody = http_body::combinators::UnsyncBoxBody<Bytes, crate::Error>;

    fn box_http_body(body: ResBody) -> Self::BoxHttpBody {
        Self::BoxHttpBody::new(body.map_err(Into::into))
    }
}

impl<F, ResBody> FutureExecutor<F, ResBody> for LocalExec
where
    Self: ResBodyExecutor<ResBody>,
    ResBody: http_body::Body<Data = Bytes> + 'static,
    ResBody::Error: Into<crate::Error>,
{
    type BoxHttpBody = crate::body::UnsendBoxBody<Bytes, crate::Error>;

    fn box_http_body(body: ResBody) -> Self::BoxHttpBody {
        Self::BoxHttpBody::new(body.map_err(Into::into))
    }
}

#[derive(Default, Copy, Clone)]
pub struct TokioExec;

impl<F> Executor<F> for TokioExec
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    fn execute(&self, fut: F) {
        tokio::spawn(fut);
    }
}

#[derive(Default, Copy, Clone)]
pub struct LocalExec;

impl<F> Executor<F> for LocalExec
where
    F: Future + 'static,
    F::Output: 'static,
{
    fn execute(&self, fut: F) {
        tokio::task::spawn_local(fut);
    }
}

#[derive(Clone)]
pub(crate) struct SharedExec {
    inner: Arc<dyn Executor<BoxFuture<'static, ()>> + Send + Sync + 'static>,
}

impl SharedExec {
    pub(crate) fn new<E>(exec: E) -> Self
    where
        E: Executor<BoxFuture<'static, ()>> + Send + Sync + 'static,
    {
        Self {
            inner: Arc::new(exec),
        }
    }

    pub(crate) fn tokio() -> Self {
        Self::new(TokioExec)
    }
}

impl Executor<BoxFuture<'static, ()>> for SharedExec {
    fn execute(&self, fut: BoxFuture<'static, ()>) {
        self.inner.execute(fut)
    }
}
