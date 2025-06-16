// cargo add tokio --features full

use tokio::io;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio::net::{TcpListener, TcpStream};
use tokio::time::{timeout, Duration};

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

    let client_to_server = async {
        loop {
            let mut buf = [0u8; 4096];
            let n = match timeout(Duration::from_secs(1), ri.read(&mut buf)).await {
                Ok(Ok(0)) => break, // EOF
                Ok(Ok(n)) => n,
                Ok(Err(e)) => return Err(e),
                Err(_) => return Err(io::Error::new(io::ErrorKind::TimedOut, "client->server timeout")),
            };
            wo.write_all(&buf[..n]).await?;
        }
        Ok::<(), io::Error>(())
    };

    let server_to_client = async {
        loop {
            let mut buf = [0u8; 4096];
            let n = match timeout(Duration::from_secs(1), ro.read(&mut buf)).await {
                Ok(Ok(0)) => break, // EOF
                Ok(Ok(n)) => n,
                Ok(Err(e)) => return Err(e),
                Err(_) => return Err(io::Error::new(io::ErrorKind::TimedOut, "server->client timeout")),
            };
            wi.write_all(&buf[..n]).await?;
        }
        Ok::<(), io::Error>(())
    };

    tokio::try_join!(client_to_server, server_to_client)?;
    Ok(())
}
