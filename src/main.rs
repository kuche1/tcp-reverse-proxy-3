// cargo add tokio --features full
// cargo add mini-redis

use std::net::SocketAddr;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::time;

const BIND: &str = "127.0.0.1:8080";
const BACKEND_SERVER: &str = "127.0.0.1:8081"; // Change to your backend server
const TIMEOUT_MS: u64 = 500;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind(BIND).await?;
    println!("Server running on: {}", BIND);
    println!("Proxying to backend: {}", BACKEND_SERVER);

    loop {
        let (client_socket, addr) = listener.accept().await?;
        println!("New connection from: {}", addr);

        tokio::spawn(async move {
            if let Err(e) = handle_client(client_socket, addr).await {
                eprintln!("Error handling client {}: {}", addr, e);
            }
        });
    }
}

async fn handle_client(
    client_socket: TcpStream,
    addr: SocketAddr,
) -> Result<(), Box<dyn std::error::Error>> {
    // Connect to backend server
    let backend_socket = TcpStream::connect(BACKEND_SERVER).await?;
    println!("Connected to backend for client {}", addr);

    // Split client and backend sockets into read/write halves
    let (mut client_read, mut client_write) = client_socket.into_split();
    let (mut backend_read, mut backend_write) = backend_socket.into_split();

    let timeout_duration = time::Duration::from_millis(TIMEOUT_MS);
    let mut deadline = time::Instant::now() + timeout_duration;

    // Create separate buffers for each direction
    let mut client_to_backend_buf = [0u8; 1024];
    let mut backend_to_client_buf = [0u8; 1024];

    loop {
        tokio::select! {
            // Timeout handler
            _ = time::sleep_until(deadline) => {
                println!("Connection timed out for {}", addr);
                break;
            }

            // Client -> Backend direction
            res = client_read.read(&mut client_to_backend_buf) => {
                let n = match res {
                    Ok(0) => break, // Client disconnected
                    Ok(n) => n,
                    Err(e) => {
                        eprintln!("Client read error: {}", e);
                        break;
                    }
                };

                // Reset timeout deadline
                deadline = time::Instant::now() + timeout_duration;

                // Forward to backend
                if let Err(e) = backend_write.write_all(&client_to_backend_buf[..n]).await {
                    eprintln!("Backend write error: {}", e);
                    break;
                }
            }

            // Backend -> Client direction
            res = backend_read.read(&mut backend_to_client_buf) => {
                let n = match res {
                    Ok(0) => break, // Backend disconnected
                    Ok(n) => n,
                    Err(e) => {
                        eprintln!("Backend read error: {}", e);
                        break;
                    }
                };

                // Reset timeout deadline
                deadline = time::Instant::now() + timeout_duration;

                // Forward to client
                if let Err(e) = client_write.write_all(&backend_to_client_buf[..n]).await {
                    eprintln!("Client write error: {}", e);
                    break;
                }
            }
        }
    }

    println!("Connection closed for {}", addr);
    Ok(())
}
