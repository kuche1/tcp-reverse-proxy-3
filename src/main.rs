// cargo add tokio --features full
// cargo add mini-redis

// use mini_redis::{Result, client};
//
// #[tokio::main]
// async fn main() -> Result<()> {
//     // Open a connection to the mini-redis address.
//     let mut client = client::connect("127.0.0.1:6379").await?;
//
//     // Set the key "hello" with value "world"
//     client.set("hello", "world".into()).await?;
//
//     // Get key "hello"
//     let result = client.get("hello").await?;
//
//     println!("got value from the server; result={:?}", result);
//
//     Ok(())
// }

use mini_redis::{Connection, Frame};
use tokio::net::{TcpListener, TcpStream};

#[tokio::main]
async fn main() {
    let listener = TcpListener::bind("127.0.0.1:6379").await.unwrap();

    loop {
        let (socket, _addr) = listener.accept().await.unwrap();
        // A new task is spawned for each inbound socket. The socket is
        // moved to the new task and processed there.
        tokio::spawn(async move {
            process(socket).await;
        });
    }
}

async fn process(socket: TcpStream) {
    // The `Connection` lets us read/write redis **frames** instead of
    // byte streams. The `Connection` type is defined by mini-redis.
    let mut connection = Connection::new(socket);

    if let Some(frame) = connection.read_frame().await.unwrap() {
        println!("GOT: {:?}", frame);

        // Respond with an error
        let response = Frame::Error("unimplemented".to_string());
        connection.write_frame(&response).await.unwrap();
    }
}
