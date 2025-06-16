use std::{error::Error, net::SocketAddr, sync::Arc};
use tokio::{
    io::{self, AsyncWriteExt},
    net::{TcpListener, TcpStream},
}; // cargo add tokio --features full

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Configure proxy settings
    let listen_addr = "0.0.0.0:8080"; // Local address to bind to
    let target_addr = "1.2.3.4:80"; // Remote server to forward to

    // Parse addresses
    let listen_addr: SocketAddr = listen_addr.parse()?;
    let target_addr: SocketAddr = target_addr.parse()?;

    // Share target address using Arc for efficient cloning
    let target_addr = Arc::new(target_addr);

    // Create TCP listener
    let listener = TcpListener::bind(listen_addr).await?;
    println!("Reverse proxy listening on: {}", listen_addr);
    println!("Forwarding traffic to: {}", target_addr);

    // Accept incoming connections
    while let Ok((mut client, client_addr)) = listener.accept().await {
        println!("Accepted connection from: {}", client_addr);

        // Clone target address for this connection
        let target_addr = Arc::clone(&target_addr);

        // Spawn task to handle connection
        tokio::spawn(async move {
            // Connect to target server
            let mut server = match TcpStream::connect(*target_addr).await {
                Ok(server) => server,
                Err(err) => {
                    eprintln!("Failed to connect to {}: {}", target_addr, err);
                    let _ = client.shutdown().await;
                    return;
                }
            };

            println!(
                "Connected to target {} for client {}",
                target_addr, client_addr
            );

            // Split client stream into read/write halves
            let (mut client_read, mut client_write) = client.split();
            // Split server stream into read/write halves
            let (mut server_read, mut server_write) = server.split();

            // Create bidirectional copy tasks
            let client_to_server = io::copy(&mut client_read, &mut server_write);
            let server_to_client = io::copy(&mut server_read, &mut client_write);

            // Wait for either direction to complete
            match tokio::try_join!(client_to_server, server_to_client) {
                Ok((from_client, from_server)) => {
                    println!(
                        "Connection closed: client={} | client->server: {} bytes, server->client: {} bytes",
                        client_addr, from_client, from_server
                    );
                }
                Err(err) => {
                    eprintln!("Connection error: {} - {}", client_addr, err);
                }
            }
        });
    }

    Ok(())
}
