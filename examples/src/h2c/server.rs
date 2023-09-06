use tonic::{transport::Server, Request, Response, Status};

use hello_world::greeter_server::{Greeter, GreeterServer};
use hello_world::{HelloReply, HelloRequest};
use tower::make::Shared;

#[cfg(not(feature = "current-thread"))]
use tonic::transport::TokioExec as Exec;
#[cfg(feature = "current-thread")]
use tonic::transport::LocalExec as Exec;

#[cfg(not(feature = "current-thread"))]
use tonic::body::BoxBody;
#[cfg(feature = "current-thread")]
use tonic::body::LocalBoxBody as BoxBody;

#[cfg(not(feature = "current-thread"))]
use tokio::spawn as spawn_task;
#[cfg(feature = "current-thread")]
use tokio::task::spawn_local as spawn_task;

pub mod hello_world {
    tonic::include_proto!("helloworld");
}

#[derive(Default)]
pub struct MyGreeter {}

#[cfg_attr(not(feature = "current-thread"), tonic::async_trait)]
#[cfg_attr(feature = "current-thread", tonic::async_trait(?Send))]
impl Greeter for MyGreeter {
    async fn say_hello(
        &self,
        request: Request<HelloRequest>,
    ) -> Result<Response<HelloReply>, Status> {
        println!("Got a request from {:?}", request.remote_addr());

        let reply = hello_world::HelloReply {
            message: format!("Hello {}!", request.into_inner().name),
        };
        Ok(Response::new(reply))
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "[::1]:50051".parse().unwrap();
    let greeter = MyGreeter::default();

    println!("GreeterServer listening on {}", addr);

    let svc = Server::builder()
        .add_service(GreeterServer::new(greeter))
        .into_router();

    let h2c = h2c::H2c { s: svc };

    let server = hyper::Server::bind(&addr).executor(Exec).serve(Shared::new(h2c));
    server.await.unwrap();

    Ok(())
}

macro_rules! define_h2c {
($($maybe_send: tt)?) => {

mod h2c {
    use super::{BoxBody, Exec, spawn_task};
    use std::pin::Pin;

    use http::{Request, Response};
    use hyper::Body;
    use tower::Service;

    #[derive(Clone)]
    pub struct H2c<S> {
        pub s: S,
    }

    type BoxError = Box<dyn std::error::Error + Send + Sync>;

    impl<S> Service<Request<Body>> for H2c<S>
    where
        S: Service<Request<Body>, Response = Response<BoxBody>>
            + Clone
            $(+ $maybe_send)*
            + 'static,
        S::Future: $($maybe_send +)* 'static,
        S::Error: Into<BoxError> + Sync + Send + 'static,
        S::Response: $($maybe_send +)* 'static,
    {
        type Response = hyper::Response<Body>;
        type Error = hyper::Error;
        type Future =
            Pin<Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> $(+ $maybe_send)*>>;

        fn poll_ready(
            &mut self,
            _: &mut std::task::Context<'_>,
        ) -> std::task::Poll<Result<(), Self::Error>> {
            std::task::Poll::Ready(Ok(()))
        }

        fn call(&mut self, mut req: hyper::Request<Body>) -> Self::Future {
            let svc = self.s.clone();
            Box::pin(async move {
                spawn_task(async move {
                    let upgraded_io = hyper::upgrade::on(&mut req).await.unwrap();

                    hyper::server::conn::Http::new()
                        .with_executor(Exec)
                        .http2_only(true)
                        .serve_connection(upgraded_io, svc)
                        .await
                        .unwrap();
                });

                let mut res = hyper::Response::new(hyper::Body::empty());
                *res.status_mut() = http::StatusCode::SWITCHING_PROTOCOLS;
                res.headers_mut().insert(
                    hyper::header::UPGRADE,
                    http::header::HeaderValue::from_static("h2c"),
                );

                Ok(res)
            })
        }
    }
}

}
}

#[cfg(not(feature = "current-thread"))]
define_h2c!(Send);
#[cfg(feature = "current-thread")]
define_h2c!();
