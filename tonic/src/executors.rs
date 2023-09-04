use std::convert::Infallible;
use std::future::Future;

use bytes::Bytes;
use http_body::combinators::UnsyncBoxBody;
use hyper::Body;
use tower::util::BoxService;
use tower_service::Service;

use crate::body::{BoxBody, LocalBoxBody, LocalBoxHttpBody, UnsendBoxBody};
use crate::body::BoxBodyExt;
use crate::server::NamedService;
use crate::util::{LocalBoxCloneService, LocalBoxService};
use http::{Response, Request};

pub trait ResBodyConstraint<ResBody> {
    fn hello();
}

impl<ResBody> ResBodyConstraint<ResBody> for LocalExecutor
where ResBody: http_body::Body<Data = Bytes> + 'static,
      ResBody::Error: Into<crate::Error> {
    fn hello() {
        panic!("gogo");
    }
}

impl<ResBody> ResBodyConstraint<ResBody> for MultiThreadExecutor
where ResBody: http_body::Body<Data = Bytes> + Copy + Send + 'static,
      ResBody::Error: Into<crate::Error> {
    fn hello() {
        panic!("bybu");
    }

}

pub trait ResBodddy {

}

impl<Resbody> ResBodddy for (MultiThreadExecutor, Resbody)
    where Resbody: http_body::Body<Data = Bytes> + Copy + Send + 'static,
          Resbody::Error: Into<crate::Error>
{

}


pub trait Executor<F, R>: Clone {
    type BoxBody: BoxBodyExt;
    type BoxCloneService;

    fn wrap_service<S>(svc: S) -> Self::BoxCloneService
    where S: Service<Request<Body>, Response = R, Future = F, Error = Infallible>
        + NamedService
        + Clone
        + Send
        + 'static,
        S::Error: Into<crate::Error> + Send;
}

#[derive(Default, Clone)]
pub struct LocalExecutor;

impl<F> hyper::rt::Executor<F> for LocalExecutor
where
    F: std::future::Future + 'static,
{
    fn execute(&self, fut: F) {
        tokio::task::spawn_local(fut);
    }
}

#[derive(Default, Clone)]
pub struct MultiThreadExecutor;

impl<F> hyper::rt::Executor<F> for MultiThreadExecutor
where
    F: std::future::Future + Send + 'static,
    F::Output: Send,
{
    fn execute(&self, fut: F) {
        tokio::spawn(fut);
    }
}

pub trait MakeBoxBody<F, ResBody>
{
    type BoxBody;

    fn make_box_body(body: ResBody) -> Self::BoxBody;
}

pub trait MakeBoxServiceLayer<S, ResBody>: MakeBoxBody<S::Future, ResBody> + Clone where
    S: Service<Request<Body>, Response = Response<ResBody>>,
    ResBody: http_body::Body<Data = Bytes>,
{
    type BoxService;
    type BoxHttpBody: http_body::Body;
}

impl<F, ResBody> MakeBoxBody<F, ResBody> for LocalExecutor
where
    ResBody: http_body::Body<Data = Bytes> + 'static,
    ResBody::Error: Into<crate::Error>,
    F: 'static,
{
    type BoxBody = UnsendBoxBody<Bytes, crate::Error>;

    fn make_box_body(body: ResBody) -> Self::BoxBody {
        UnsendBoxBody::new(body.map_err(Into::into))
    }
}

impl<S, ResBody> MakeBoxServiceLayer<S, ResBody> for LocalExecutor
where
    S: Service<Request<Body>, Response = Response<ResBody>> + Clone + Send + 'static,
    S::Error: Into<crate::Error> + Send,
    ResBody: http_body::Body<Data = Bytes> + 'static,
    ResBody::Error: Into<crate::Error>
{
    type BoxService = LocalBoxService<Request<Body>, Response<Self::BoxHttpBody>, crate::Error>;
    type BoxHttpBody = LocalBoxHttpBody;
}

impl<F, ResBody> MakeBoxBody<F, ResBody> for MultiThreadExecutor
where
    ResBody: http_body::Body<Data = Bytes> + 'static + Send,
    ResBody::Error: Into<crate::Error>,
    F: 'static + Send,
{
    type BoxBody = UnsyncBoxBody<Bytes, crate::Error>;

    fn make_box_body(body: ResBody) -> Self::BoxBody {
        UnsyncBoxBody::new(body.map_err(Into::into))
    }
}

impl<S, ResBody> MakeBoxServiceLayer<S, ResBody> for MultiThreadExecutor
where
    S: Service<Request<Body>, Response = Response<ResBody>> + Clone + Send + 'static,
    S::Future: Send + 'static,
    S::Error: Into<crate::Error> + Send,
    ResBody: http_body::Body<Data = Bytes> + Send + 'static,
    ResBody::Error: Into<crate::Error>
{
    type BoxService = BoxService<Request<Body>, Response<Self::BoxHttpBody>, crate::Error>;
    type BoxHttpBody = UnsyncBoxBody<Bytes, crate::Error>;
}

impl<F> Executor<F, Response<LocalBoxBody>> for LocalExecutor
where F: Future + 'static {
//    R: http_body::Body<Data = bytes::Bytes, Error = crate::Status> + 'static {
    type BoxBody = LocalBoxBody;
    type BoxCloneService = LocalBoxCloneService<Request<Body>, Response<LocalBoxBody>, Infallible>;

    fn wrap_service<S>(svc: S) -> Self::BoxCloneService
        where S: Service<Request<Body>, Response = Response<LocalBoxBody>, Future = F, Error = Infallible>
            + NamedService
            + Clone
            + Send
            + 'static,
            S::Error: Into<crate::Error> + Send {
        LocalBoxCloneService::new(svc)
    }
}

impl<F> Executor<F, Response<BoxBody>> for MultiThreadExecutor
where F: Future + Send + 'static {
//    R: http_body::Body<Data = bytes::Bytes, Error = crate::Status> + Send + 'static {
    type BoxBody = BoxBody;
    type BoxCloneService = tower::util::BoxCloneService<Request<Body>, Response<BoxBody>, Infallible>;

    fn wrap_service<S>(svc: S) -> Self::BoxCloneService
        where S: Service<Request<Body>, Response = Response<BoxBody>, Future = F, Error = Infallible>
            + NamedService
            + Clone
            + Send
            + 'static,
            S::Error: Into<crate::Error> + Send {
        tower::util::BoxCloneService::new(svc)
    }
}
