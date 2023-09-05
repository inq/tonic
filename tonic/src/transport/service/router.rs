use crate::body::LocalBoxBody;
use crate::codegen::{BoxBodyExt, BoxFuture, LocalBoxFuture};
use crate::body::BoxBody;
use http::{Request, Response};
use hyper::Body;
use std::{
    convert::Infallible,
    fmt,
    task::{Context, Poll},
};
use tower_service::Service;

/// A [`Service`] router.
#[derive(Clone)]
pub struct Routes<S = BoxCloneService> {
    router: matchit::Router<S>,
}

impl<S> Default for Routes<S> {
    fn default() -> Self {
        Self { router: Default::default() }
    }
}

impl<S> Routes<S> {
    /// Create a new routes with `svc` already added to it.
    pub fn new(path: &str, svc: S) -> Self
    {
        let router = matchit::Router::default();
        let mut res = Self { router };
        res.add_service(path, svc);
        res
    }

    /// Add a new service.
    pub fn add_service(&mut self, path: &str, svc: S) -> &mut Self {
        self
            .router
            .insert(path, svc)
            .unwrap();
        self
    }
}

impl<S> fmt::Debug for Routes<S> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Routes").finish()
    }
}

type BoxCloneService = tower::util::BoxCloneService<Request<Body>, Response<BoxBody>, Infallible>;
impl Service<Request<Body>> for Routes<BoxCloneService>
{
    type Response = Response<BoxBody>;
    type Error = Infallible;
    type Future = BoxFuture<Self::Response, Self::Error>;

    #[inline]
    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Infallible>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        if let Ok(matched) = self.router.at(req.uri().path()) {
            matched.value.clone().call(req)
        } else {
            Box::pin(
                async move {
                    Ok(
                    Response::builder()
                    .status(http::StatusCode::OK)
                    .header("grpc-status", "12")
                    .header("content-type", "application/grpc")
                    .body(BoxBody::empty_body())
                    .unwrap()
                    )
            })
        }
    }
}

type LocalBoxCloneService = crate::util::LocalBoxCloneService<Request<Body>, Response<LocalBoxBody>, Infallible>;
impl Service<Request<Body>> for Routes<LocalBoxCloneService>
{
    type Response = Response<LocalBoxBody>;
    type Error = Infallible;
    type Future = LocalBoxFuture<Self::Response, Self::Error>;

    #[inline]
    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Infallible>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        if let Ok(matched) = self.router.at(req.uri().path()) {
            matched.value.clone().call(req)
        } else {
            Box::pin(
                async move {
                    Ok(
                    Response::builder()
                    .status(http::StatusCode::OK)
                    .header("grpc-status", "12")
                    .header("content-type", "application/grpc")
                    .body(LocalBoxBody::empty_body())
                    .unwrap()
                    )
            })
        }
    }
}
