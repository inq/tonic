#![allow(unused_imports)]

#[cfg(not(feature = "current-thread"))]
use std::sync::Arc as MaybeArc;
#[cfg(feature = "current-thread")]
use std::rc::Rc as MaybeArc;
use tokio_stream::{Stream, StreamExt};
use tonic::{Request, Response, Status};

tonic::include_proto!("test");

#[derive(Debug, Default)]
struct Svc;

#[cfg_attr(not(feature = "current-thread"), tonic::async_trait)]
#[cfg_attr(feature = "current-thread", tonic::async_trait(?Send))]
impl test_server::Test for Svc {
    async fn test_request(
        self: MaybeArc<Self>,
        req: Request<SomeData>,
    ) -> Result<Response<SomeData>, Status> {
        Ok(Response::new(req.into_inner()))
    }
}
