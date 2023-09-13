#![allow(unused_imports)]

use tokio_stream::{Stream, StreamExt};
use tonic::{Request, Response, Status};

tonic::include_proto!("test");

#[derive(Debug, Default)]
struct Svc;

#[tonic::async_trait(?Send)]
impl test_server::Test for Svc {
    async fn test_request(
        &self,
        req: Request<SomeData>,
    ) -> Result<Response<SomeData>, Status> {
        Ok(Response::new(req.into_inner()))
    }
}
