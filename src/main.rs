// cargo add tokio --features full
// cargo add mini-redis

use std::net::SocketAddr;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::net::TcpStream;
use tokio::time;

const TIMEOUT: u64 = 1;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind("127.0.0.1:8080").await?;
    println!("Server running on 127.0.0.1:8080");

    loop {
        let (socket, addr) = listener.accept().await?;
        println!("New connection from: {}", addr);

        tokio::spawn(async move {
            handle_client(socket, addr).await;
        });
    }
}

async fn handle_client(mut socket: TcpStream, addr: SocketAddr) {
    let mut buf = [0; 1024];
    let timeout_duration = time::Duration::from_secs(TIMEOUT); // Set timeout duration (e.g., 30 seconds)

    loop {
        // Wrap the read operation in a timeout
        match time::timeout(timeout_duration, socket.read(&mut buf)).await {
            // Read completed within timeout
            Ok(Ok(0)) => {
                // Client disconnected
                println!("Connection closed by {}", addr);
                return;
            }
            Ok(Ok(n)) => {
                // Data received
                if let Err(e) = socket.write_all(&buf[0..n]).await {
                    eprintln!("Write error to {}: {}", addr, e);
                    return;
                }
            }
            Ok(Err(e)) => {
                // Read error
                eprintln!("Read error from {}: {}", addr, e);
                return;
            }
            Err(_) => {
                // Timeout elapsed
                println!(
                    "Connection timed out (no data for {:?}) for {}",
                    timeout_duration, addr
                );
                return;
            }
        }
    }
}
