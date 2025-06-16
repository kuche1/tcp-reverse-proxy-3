use rustls::{ServerConfig};
use rustls_pemfile::{certs, rsa_private_keys};
use std::fs::File;
use std::io::BufReader;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt}; // cargo add tokio --features full
use tokio::net::TcpListener;
use tokio::time::{Duration, timeout};
use tokio_rustls::TlsAcceptor; // cargo add tokio-rustls rustls rustls-pemfile
use rustls::pki_types::{CertificateDer, PrivatePkcs1KeyDer, PrivateKeyDer};

const PORT: u16 = 34446;
const TIMEOUT_MS: u64 = 1_000;
const REMOTE_ADDR: &str = "127.0.0.1:32850";

fn load_certs(path: &str) -> Vec<CertificateDer<'static>> {
    let mut reader = BufReader::new(File::open(path).expect("cannot open cert.pem"));
    certs(&mut reader)
        .map(|res| res.expect("cannot read cert"))
        .collect()
}

fn load_key(path: &str) -> PrivateKeyDer<'static> {
    let mut reader = BufReader::new(File::open(path).expect("cannot open key.pem"));
    // Try PKCS#1 first
    let mut keys = rsa_private_keys(&mut reader)
        .map(|res| res.expect("cannot read private key"))
        .map(PrivatePkcs1KeyDer::into);
    if let Some(key) = keys.next() {
        return key;
    }
    // If PKCS#1 fails, try PKCS#8
    let mut reader = BufReader::new(File::open(path).expect("cannot open key.pem"));
    let pkcs8_keys = rustls_pemfile::pkcs8_private_keys(&mut reader)
        .map(|res| res.expect("cannot read pkcs8 private key"));
    if let Some(key) = pkcs8_keys.map(PrivateKeyDer::from).next() {
        return key;
    }
    panic!("no private key found in {}", path);
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    // TLS setup
    let certs = load_certs("cert.pem");
    let key = load_key("key.pem");
    let config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .expect("bad certificates/private key");

    let acceptor = TlsAcceptor::from(Arc::new(config));

    let listener = TcpListener::bind(("0.0.0.0", PORT)).await?;
    println!("TLS proxy server listening on 0.0.0.0:{}", PORT);

    loop {
        let (inbound, addr) = listener.accept().await?;
        let acceptor = acceptor.clone();
        println!("Accepted connection from {}", addr);

        tokio::spawn(async move {
            match acceptor.accept(inbound).await {
                Ok(mut tls_stream) => {
                    // Add timeout to the connect
                    match timeout(
                        Duration::from_millis(TIMEOUT_MS),
                        tokio::net::TcpStream::connect(REMOTE_ADDR)
                    ).await {
                        Ok(Ok(mut outbound)) => {
                            println!("Connected to remote {}", REMOTE_ADDR);
                            if let Err(e) = proxy(&mut tls_stream, &mut outbound).await {
                                eprintln!("proxy error: {}", e);
                            }
                        }
                        Ok(Err(e)) => {
                            eprintln!("Failed to connect to remote {}: {}", REMOTE_ADDR, e);
                        }
                        Err(_) => {
                            eprintln!("Timeout connecting to remote {}", REMOTE_ADDR);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("TLS handshake failed: {}", e);
                }
            }
        });
    }
}

async fn proxy<S1, S2>(inbound: &mut S1, outbound: &mut S2) -> std::io::Result<()>
where
    S1: AsyncReadExt + AsyncWriteExt + Unpin,
    S2: AsyncReadExt + AsyncWriteExt + Unpin,
{
    use tokio::io::split;
    let (mut ri, mut wi) = split(inbound);
    let (mut ro, mut wo) = split(outbound);

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

    Ok(())
}
