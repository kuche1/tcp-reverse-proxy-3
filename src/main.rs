use tokio::io::{AsyncReadExt, AsyncWriteExt}; // cargo add tokio --features full
use tokio::net::TcpListener;
use tokio::time::{Duration, timeout};

const PORT: u16 = 34446;
const TIMEOUT_MS: u64 = 5_000;
const REMOTE_ADDR: &str = "127.0.0.1:32850";

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind(("0.0.0.0", PORT)).await?;
    println!("Proxy server listening on 0.0.0.0:{}", PORT);

    loop {
        let (mut inbound, addr) = listener.accept().await?;
        println!("Accepted connection from {}", addr);

        tokio::spawn(async move {
            match tokio::net::TcpStream::connect(REMOTE_ADDR).await {
                Ok(mut outbound) => {
                    println!("Connected to remote {}", REMOTE_ADDR);
                    if let Err(e) = proxy(&mut inbound, &mut outbound).await {
                        eprintln!("proxy error: {}", e);
                    }
                }
                Err(e) => {
                    eprintln!("Failed to connect to remote {}: {}", REMOTE_ADDR, e);
                }
            }
        });
    }
}

async fn proxy(
    inbound: &mut tokio::net::TcpStream,
    outbound: &mut tokio::net::TcpStream,
) -> std::io::Result<()> {
    use tokio::io::split;
    let (mut ri, mut wi) = split(&mut *inbound);
    let (mut ro, mut wo) = split(&mut *outbound);

    let client_to_server = async {
        let mut buf = [0u8; 1024];
        loop {
            let n = match timeout(Duration::from_millis(TIMEOUT_MS), ri.read(&mut buf)).await {
                Ok(Ok(0)) => break,
                Ok(Ok(n)) => n,
                Ok(Err(e)) => {
                    eprintln!("read error (client->server): {}", e);
                    break;
                }
                Err(_) => {
                    eprintln!("timeout: no data received from client in {} ms", TIMEOUT_MS);
                    break;
                }
            };
            if let Err(e) = wo.write_all(&buf[..n]).await {
                eprintln!("write error (client->server): {}", e);
                break;
            }
        }
        Ok::<(), std::io::Error>(())
    };

    let server_to_client = async {
        let mut buf = [0u8; 1024];
        loop {
            let n = match timeout(Duration::from_millis(TIMEOUT_MS), ro.read(&mut buf)).await {
                Ok(Ok(0)) => break,
                Ok(Ok(n)) => n,
                Ok(Err(e)) => {
                    eprintln!("read error (server->client): {}", e);
                    break;
                }
                Err(_) => {
                    eprintln!("timeout: no data received from server in {} ms", TIMEOUT_MS);
                    break;
                }
            };
            if let Err(e) = wi.write_all(&buf[..n]).await {
                eprintln!("write error (server->client): {}", e);
                break;
            }
        }
        Ok::<(), std::io::Error>(())
    };

    tokio::try_join!(client_to_server, server_to_client)?;

    // Explicitly shutdown both sockets
    inbound.shutdown().await.ok();
    outbound.shutdown().await.ok();

    Ok(())
}
