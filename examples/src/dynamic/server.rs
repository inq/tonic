use std::env;
use tonic::{transport::Server, Request, Response, Status};

use hello_world::greeter_server::{Greeter, GreeterServer};
use hello_world::{HelloReply, HelloRequest};

use echo::echo_server::{Echo, EchoServer};
use echo::{EchoRequest, EchoResponse};

#[cfg(not(feature = "current-thread"))]
use tonic::transport::server::Routes as Routes;
#[cfg(feature = "current-thread")]
use tonic::transport::server::LocalRoutes as Routes;

pub mod hello_world {
    tonic::include_proto!("helloworld");
}

pub mod echo {
    tonic::include_proto!("grpc.examples.unaryecho");
}

type EchoResult<T> = Result<Response<T>, Status>;

#[derive(Default)]
pub struct MyEcho {}

#[cfg_attr(not(feature = "current-thread"), tonic::async_trait)]
#[cfg_attr(feature = "current-thread", tonic::async_trait(?Send))]
impl Echo for MyEcho {
    async fn unary_echo(&self, request: Request<EchoRequest>) -> EchoResult<EchoResponse> {
        println!("Got an echo request from {:?}", request.remote_addr());

        let message = format!("you said: {}", request.into_inner().message);

        Ok(Response::new(EchoResponse { message }))
    }
}

fn init_echo(args: &[String], routes: &mut Routes) {
    let enabled = args
        .into_iter()
        .find(|arg| arg.as_str() == "echo")
        .is_some();
    if enabled {
        println!("Adding Echo service...");
        let svc = EchoServer::new(MyEcho::default());
        routes.add_service(svc);
    }
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
        println!("Got a greet request from {:?}", request.remote_addr());

        let reply = hello_world::HelloReply {
            message: format!("Hello {}!", request.into_inner().name),
        };
        Ok(Response::new(reply))
    }
}

fn init_greeter(args: &[String], routes: &mut Routes) {
    let enabled = args
        .into_iter()
        .find(|arg| arg.as_str() == "greeter")
        .is_some();

    if enabled {
        println!("Adding Greeter service...");
        let svc = GreeterServer::new(MyGreeter::default());
        routes.add_service(svc);
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    let mut routes_builder = Routes::default();
    init_greeter(&args, &mut routes_builder);
    init_echo(&args, &mut routes_builder);

    let addr = "[::1]:50051".parse().unwrap();

    println!("Grpc server listening on {}", addr);

    #[cfg(not(feature = "current-thread"))]
    let mut builder = Server::builder();
    #[cfg(feature = "current-thread")]
    let mut builder = Server::builder().current_thread_executor();
    builder
        .add_routes(routes_builder)
        .serve(addr)
        .await?;

    Ok(())
}
