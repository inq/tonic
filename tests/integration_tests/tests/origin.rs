use integration_tests::pb::test_client;
use integration_tests::pb::{test_server, Input, Output};
use std::task::Context;
use std::task::Poll;
use std::time::Duration;
use tokio::sync::oneshot;
use tonic::codegen::http::Request;
use tonic::{
    transport::{Endpoint, Server},
    Response, Status,
};
use tower::Layer;
use tower::Service;

#[cfg(not(feature = "current-thread"))]
use tokio::spawn as spawn_task;
#[cfg(feature = "current-thread")]
use tokio::task::spawn_local as spawn_task;

#[cfg(not(feature = "current-thread"))]
use integration_tests::BoxFuture;
#[cfg(feature = "current-thread")]
use integration_tests::LocalBoxFuture as BoxFuture;

#[tonic_test::test]
async fn writes_origin_header() {
    struct Svc;

    #[cfg_attr(not(feature = "current-thread"), tonic::async_trait)]
    #[cfg_attr(feature = "current-thread", tonic::async_trait(?Send))]
    impl test_server::Test for Svc {
        async fn unary_call(
            &self,
            _req: tonic::Request<Input>,
        ) -> Result<Response<Output>, Status> {
            Ok(Response::new(Output {}))
        }
    }

    let svc = test_server::TestServer::new(Svc);

    let (tx, rx) = oneshot::channel::<()>();

    let jh = spawn_task(async move {
        #[cfg(not(feature = "current-thread"))]
        let mut builder = Server::builder();
        #[cfg(feature = "current-thread")]
        let mut builder = Server::builder().local_executor();
        builder
            .layer(OriginLayer {})
            .add_service(svc)
            .serve_with_shutdown("127.0.0.1:1442".parse().unwrap(), async { drop(rx.await) })
            .await
            .unwrap();
    });

    tokio::time::sleep(Duration::from_millis(100)).await;

    let channel = Endpoint::from_static("http://127.0.0.1:1442")
        .origin("https://docs.rs".parse().expect("valid uri"))
        .connect()
        .await
        .unwrap();

    let mut client = test_client::TestClient::new(channel);

    match client.unary_call(Input {}).await {
        Ok(_) => {}
        Err(status) => panic!("{}", status.message()),
    }

    tx.send(()).unwrap();

    jh.await.unwrap();
}

#[derive(Clone)]
struct OriginLayer {}

impl<S> Layer<S> for OriginLayer {
    type Service = OriginService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        OriginService { inner }
    }
}

#[derive(Clone)]
struct OriginService<S> {
    inner: S,
}

macro_rules! define_origin_service {
($($maybe_send: tt)?) => {

impl<T> Service<Request<tonic::transport::Body>> for OriginService<T>
where
    T: Service<Request<tonic::transport::Body>>,
    T::Future: $($maybe_send +)* 'static,
    T::Error: Into<Box<dyn std::error::Error + Send + Sync>>,
{
    type Response = T::Response;
    type Error = Box<dyn std::error::Error + Send + Sync>;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx).map_err(Into::into)
    }

    fn call(&mut self, req: Request<tonic::transport::Body>) -> Self::Future {
        assert_eq!(req.uri().host(), Some("docs.rs"));
        let fut = self.inner.call(req);

        Box::pin(async move { fut.await.map_err(Into::into) })
    }
}

}
}

#[cfg(not(feature = "current-thread"))]
define_origin_service!(Send);
#[cfg(feature = "current-thread")]
define_origin_service!();
