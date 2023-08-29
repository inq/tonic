use crate::codegen::empty_body;
use crate::{
    body::BoxBody,
    server::NamedService,
};
use futures_util::future::BoxFuture;
use http::{Request, Response};
use hyper::Body;
use pin_project::pin_project;
use tower::util::BoxCloneService;
use std::{
    convert::Infallible,
    fmt,
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};
use tower_service::Service;

/// A [`Service`] router.
#[derive(Default, Clone)]
pub struct Routes {
    router: matchit::Router<BoxCloneService<Request<Body>, Response<BoxBody>, Infallible>>,
}

#[derive(Default, Debug, Clone)]
/// Allows adding new services to routes by passing a mutable reference to this builder.
pub struct RoutesBuilder {
    routes: Option<Routes>,
}

impl RoutesBuilder {
    /// Add a new service.
    pub fn add_service<S>(&mut self, svc: S) -> &mut Self
    where
        S: Service<Request<Body>, Response = Response<BoxBody>, Error = Infallible>
            + NamedService
            + Clone
            + Send
            + 'static,
        S::Future: Send + 'static,
        S::Error: Into<crate::Error> + Send,
    {
        let routes = self.routes.take().unwrap_or_default();
        self.routes.replace(routes.add_service(svc));
        self
    }

    /// Returns the routes with added services or empty [`Routes`] if no service was added
    pub fn routes(self) -> Routes {
        self.routes.unwrap_or_default()
    }
}

impl Routes {
    /// Create a new routes with `svc` already added to it.
    pub fn new<S>(svc: S) -> Self
    where
        S: Service<Request<Body>, Response = Response<BoxBody>, Error = Infallible>
            + NamedService
            + Clone
            + Send
            + 'static,
        S::Future: Send + 'static,
        S::Error: Into<crate::Error> + Send,
    {
        let router = matchit::Router::default();
        Self { router }.add_service(svc)
    }

    /// Add a new service.
    pub fn add_service<S>(mut self, svc: S) -> Self
    where
        S: Service<Request<Body>, Response = Response<BoxBody>, Error = Infallible>
            + NamedService
            + Clone
            + Send
            + 'static,
        S::Future: Send + 'static,
        S::Error: Into<crate::Error> + Send,
    {
        self
            .router
            .insert(&format!("/{}/*rest", S::NAME), BoxCloneService::new(svc))
            .unwrap();
        self
    }
}

impl fmt::Debug for Routes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Routes").finish()
    }
}

impl Service<Request<Body>> for Routes {
    type Response = Response<BoxBody>;
    type Error = Infallible;
    type Future = RoutesFuture;

    #[inline]
    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Infallible>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        if let Ok(matched) = self.router.at(req.uri().path()) {
            RoutesFuture(matched.value.clone().call(req))
        } else {
            RoutesFuture(Box::pin(async move {
                Ok(Response::builder()
                    .status(http::StatusCode::OK)
                    .header("grpc-status", "12")
                    .header("content-type", "application/grpc")
                    .body(empty_body())
                    .unwrap())
            }))
        }
    }
}

#[pin_project]
pub struct RoutesFuture(#[pin] pub BoxFuture<'static, Result<Response<BoxBody>, Infallible>>);

impl fmt::Debug for RoutesFuture {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("RoutesFuture").finish()
    }
}

impl Future for RoutesFuture {
    type Output = Result<Response<BoxBody>, Infallible>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.project().0.poll(cx)
    }
}
