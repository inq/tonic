use crate::server::NamedService;
use http::{Request, Response};
use hyper::Body;
use std::{
    convert::Infallible,
    fmt,
    task::{Context, Poll},
};
use tower_service::Service;

#[cfg(not(feature = "current-thread"))]
type BoxBody = crate::body::BoxBody;
#[cfg(feature = "current-thread")]
type BoxBody = crate::body::LocalBoxBody;

#[cfg(not(feature = "current-thread"))]
type BoxCloneService = tower::util::BoxCloneService<Request<Body>, Response<BoxBody>, Infallible>;
#[cfg(feature = "current-thread")]
type BoxCloneService = crate::util::LocalBoxCloneService<Request<Body>, Response<BoxBody>, Infallible>;

#[cfg(not(feature = "current-thread"))]
type BoxFuture<T, E> = crate::codegen::BoxFuture<T, E>;
#[cfg(feature = "current-thread")]
type BoxFuture<T, E> = crate::codegen::LocalBoxFuture<T, E>;

#[cfg(not(feature = "current-thread"))]
use crate::body::empty_body;
#[cfg(feature = "current-thread")]
use crate::body::local_empty_body as empty_body;

/// A [`Service`] router.
#[derive(Default, Clone)]
pub struct Routes {
    router: matchit::Router<BoxCloneService>,
}

macro_rules! register_routers {
($($maybe_send: tt)?) => {

impl Routes {
    /// Create a new routes with `svc` already added to it.
    pub fn new<S>(svc: S) -> Self
    where
        S: Service<Request<Body>, Response = Response<BoxBody>, Error = Infallible>
            + NamedService
            + Clone
            $(+ $maybe_send)*
            + 'static,
        S::Future: $($maybe_send +)* 'static,
        S::Error: Into<crate::Error> + Send,
    {
        let router = matchit::Router::default();
        let mut res = Self { router };
        res.add_service(svc);
        res
    }

    /// Add a new service.
    pub fn add_service<S>(&mut self, svc: S) -> &mut Self
    where
        S: Service<Request<Body>, Response = Response<BoxBody>, Error = Infallible>
            + NamedService
            + Clone
            $(+ $maybe_send)*
            + 'static,
        S::Future: $($maybe_send +)* 'static,
        S::Error: Into<crate::Error> + Send,
    {
        self
            .router
            .insert(format!("/{}/*rest", S::NAME), BoxCloneService::new(svc))
            .unwrap();
        self
    }
}

}
}

#[cfg(not(feature = "current-thread"))]
register_routers!(Send);
#[cfg(feature = "current-thread")]
register_routers!();

impl fmt::Debug for Routes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Routes").finish()
    }
}

impl Service<Request<Body>> for Routes
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
                    .body(empty_body())
                    .unwrap()
                    )
            })
        }
    }
}
