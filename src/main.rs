use tokio::io::{AsyncReadExt, AsyncWriteExt}; // cargo add tokio --features full
use tokio::net::TcpListener;

const PORT: u16 = 34446;

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
        let n = match socket.read(&mut buf).await {
            Ok(0) => break,
            Ok(n) => n,
            Err(e) => {
                eprintln!("read error: {}", e);
                break;
            }
        };
        if let Err(e) = socket.write_all(&buf[..n]).await {
            eprintln!("write error: {}", e);
            break;
        }
    }
}
