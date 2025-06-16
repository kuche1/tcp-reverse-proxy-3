// cargo add tokio --features full

use tokio::io;
use tokio::net::{TcpListener, TcpStream};

#[tokio::main]
async fn main() -> io::Result<()> {
    let listen_addr = "0.0.0.0:34446";
    let remote_addr = "127.0.0.1:32850";

    let listener = TcpListener::bind(listen_addr).await?;
    println!("Listening on {}", listen_addr);

    loop {
        let (inbound, client_addr) = listener.accept().await?;
        let remote_addr = remote_addr.to_string();
        tokio::spawn(async move {
            match TcpStream::connect(&remote_addr).await {
                Ok(outbound) => {
                    println!("Proxying {} <-> {}", client_addr, remote_addr);
                    if let Err(e) = transfer(inbound, outbound).await {
                        eprintln!("Transfer error: {}", e);
                    }
                }
                Err(e) => {
                    eprintln!("Failed to connect to remote {}: {}", remote_addr, e);
                }
            }
        });
    }
}

async fn transfer(mut inbound: TcpStream, mut outbound: TcpStream) -> io::Result<()> {
    let (mut ri, mut wi) = inbound.split();
    let (mut ro, mut wo) = outbound.split();

    let client_to_server = io::copy(&mut ri, &mut wo);
    let server_to_client = io::copy(&mut ro, &mut wi);

    tokio::try_join!(client_to_server, server_to_client)?;
    Ok(())
}
