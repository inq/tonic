use tonic::{
    transport::{Endpoint, Server, Uri},
    Request, Response, Status,
};
use tower::service_fn;

pub mod hello_world {
    tonic::include_proto!("helloworld");
}

use hello_world::{
    greeter_client::GreeterClient,
    greeter_server::{Greeter, GreeterServer},
    HelloReply, HelloRequest,
};

#[cfg(not(feature = "current-thread"))]
use tokio::spawn as spawn_task;
#[cfg(feature = "current-thread")]
use tokio::task::spawn_local as spawn_task;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (client, server) = tokio::io::duplex(1024);

    let greeter = MyGreeter::default();

    spawn_task(async move {
        Server::builder()
            .add_service(GreeterServer::new(greeter))
            .serve_with_incoming(tokio_stream::iter(vec![Ok::<_, std::io::Error>(server)]))
            .await
    });

    // Move client to an option so we can _move_ the inner value
    // on the first attempt to connect. All other attempts will fail.
    let mut client = Some(client);
    let channel = Endpoint::try_from("http://[::]:50051")?
        .connect_with_connector(service_fn(move |_: Uri| {
            let client = client.take();

            async move {
                if let Some(client) = client {
                    Ok(client)
                } else {
                    Err(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "Client already taken",
                    ))
                }
            }
        }))
        .await?;

    let mut client = GreeterClient::new(channel);

    let request = tonic::Request::new(HelloRequest {
        name: "Tonic".into(),
    });

    let response = client.say_hello(request).await?;

    println!("RESPONSE={:?}", response);

    Ok(())
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
        println!("Got a request: {:?}", request);

        let reply = hello_world::HelloReply {
            message: format!("Hello {}!", request.into_inner().name),
        };
        Ok(Response::new(reply))
    }
}
