use hyper::Body;
use std::{
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};
use tonic::{transport::Server, Request, Response, Status};
use tower::{Layer, Service};

use hello_world::greeter_server::{Greeter, GreeterServer};
use hello_world::{HelloReply, HelloRequest};

pub mod hello_world {
    tonic::include_proto!("helloworld");
}

#[cfg(not(feature = "current-thread"))]
use tonic::body::BoxBody;
#[cfg(feature = "current-thread")]
use tonic::body::LocalBoxBody as BoxBody;

#[cfg(not(feature = "current-thread"))]
type BoxFuture<'a, T> = Pin<Box<dyn std::future::Future<Output = T> + Send + 'a>>;
#[cfg(feature = "current-thread")]
type BoxFuture<'a, T> = Pin<Box<dyn std::future::Future<Output = T> + 'a>>;

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

    let svc = GreeterServer::new(greeter);

    // The stack of middleware that our service will be wrapped in
    let layer = tower::ServiceBuilder::new()
        // Apply middleware from tower
        .timeout(Duration::from_secs(30))
        // Apply our own middleware
        .layer(MyMiddlewareLayer::default())
        // Interceptors can be also be applied as middleware
        .layer(tonic::service::interceptor(intercept))
        .into_inner();

    Server::builder()
        // Wrap all services in the middleware stack
        .layer(layer)
        .add_service(svc)
        .serve(addr)
        .await?;

    Ok(())
}

// An interceptor function.
fn intercept(req: Request<()>) -> Result<Request<()>, Status> {
    Ok(req)
}

#[derive(Debug, Clone, Default)]
struct MyMiddlewareLayer;

impl<S> Layer<S> for MyMiddlewareLayer {
    type Service = MyMiddleware<S>;

    fn layer(&self, service: S) -> Self::Service {
        MyMiddleware { inner: service }
    }
}

#[derive(Debug, Clone)]
struct MyMiddleware<S> {
    inner: S,
}

macro_rules! define_my_middleware {
($($maybe_send: tt)?) => {

impl<S> Service<hyper::Request<Body>> for MyMiddleware<S>
where
    S: Service<hyper::Request<Body>, Response = hyper::Response<BoxBody>> + Clone + $($maybe_send +)* 'static,
    S::Future: $($maybe_send +)* 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: hyper::Request<Body>) -> Self::Future {
        // This is necessary because tonic internally uses `tower::buffer::Buffer`.
        // See https://github.com/tower-rs/tower/issues/547#issuecomment-767629149
        // for details on why this is necessary
        let clone = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, clone);

        Box::pin(async move {
            // Do extra async work here...
            let response = inner.call(req).await?;

            Ok(response)
        })
    }
}

}
}

#[cfg(not(feature = "current-thread"))]
define_my_middleware!(Send);
#[cfg(feature = "current-thread")]
define_my_middleware!();
