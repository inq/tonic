use std::convert::Infallible;

use http::{Request, Response};
use hyper::Body;
use tower::util::BoxCloneService;
use tower_service::Service;

use crate::body::BoxBody;

#[derive(Default, Clone)]
pub struct SimpleRouter {
    inner: matchit::Router<BoxCloneService<Request<Body>, Response<BoxBody>, Infallible>>,
}

impl SimpleRouter {
    pub fn route_service<S>(&mut self, route: &str, svc: S)
    where
        S: Service<Request<Body>, Response = Response<BoxBody>, Error = Infallible> + Clone + Send + 'static,
        S::Future: 'static + Send,
    {
        self.inner.insert(route, BoxCloneService::new(svc));
    }
}