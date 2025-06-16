use tokio::io::{AsyncReadExt, AsyncWriteExt}; // cargo add tokio --features full
use tokio::net::TcpListener;
use tokio::time::{timeout, Duration};

const PORT: u16 = 34446;
const TIMEOUT_MS: u64 = 5_000;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind(("0.0.0.0", PORT)).await?;
    println!("Echo server listening on 0.0.0.0:{}", PORT);

    loop {
        let (mut socket, addr) = listener.accept().await?;

        println!("Accepted connection from {}", addr);

        tokio::spawn(async move {
            handle_client(&mut socket).await;
        });
    }
}

async fn handle_client(socket: &mut tokio::net::TcpStream) {
    let mut buf = [0u8; 1024];
    loop {
        let n = match timeout(Duration::from_millis(TIMEOUT_MS), socket.read(&mut buf)).await {
            Ok(Ok(0)) => break,
            Ok(Ok(n)) => n,
            Ok(Err(e)) => {
                eprintln!("read error: {}", e);
                break;
            }
            Err(_) => {
                eprintln!("timeout: no data received in {} ms", TIMEOUT_MS);
                break;
            }
        };
        if let Err(e) = socket.write_all(&buf[..n]).await {
            eprintln!("write error: {}", e);
            break;
        }
    }
}
