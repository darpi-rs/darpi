use std::convert::Infallible;

use std::sync::Arc;
use tokio::net::TcpListener;

use tokio_rustls::TlsAcceptor;

use hyper::server::conn::Http;
use hyper::service::service_fn;
use hyper::{Body, Request, Response};

type AnyError = Box<dyn std::error::Error + Send + Sync>;

#[tokio::main]
async fn main() -> Result<(), AnyError> {
    let rustls_config = Arc::new(rustls_server_config("key.pem", "cert.pem")?);

    let acceptor = TlsAcceptor::from(rustls_config);
    let listener = TcpListener::bind("127.0.0.1:3443").await?;

    loop {
        let (stream, _) = listener.accept().await?;
        let acceptor = acceptor.clone();

        tokio::spawn(async move {
            if let Ok(stream) = acceptor.accept(stream).await {
                let fut = Http::new().serve_connection(stream, service_fn(serve));

                match fut.await {
                    Ok(()) => (),
                    Err(_) => (),
                }
            }
        });
    }
}

async fn serve(_req: Request<Body>) -> Result<Response<Body>, Infallible> {
    let resp = Response::builder()
        .body(Body::from("Hello, world!"))
        .unwrap();

    Ok(resp)
}

///////////utils

use rustls::internal::pemfile::{certs, pkcs8_private_keys};
use rustls::{Certificate, NoClientAuth, PrivateKey, ServerConfig};
use std::fs::File;
use std::io::BufReader;

pub fn rustls_server_config(key: &str, cert: &str) -> Result<ServerConfig, AnyError> {
    let mut config = ServerConfig::new(NoClientAuth::new());

    let mut key_reader = get_file_reader(key)?;
    let mut cert_reader = get_file_reader(cert)?;

    let key = get_private_key(&mut key_reader)?;
    let certs = get_cert_chain(&mut cert_reader)?;

    config
        .set_single_cert(certs, key)
        .map_err(|_| "Invalid certificate chain or private key.")?;

    config.set_protocols(&[b"h2".to_vec(), b"http/1.1".to_vec()]);

    Ok(config)
}

fn get_cert_chain(reader: &mut BufReader<File>) -> Result<Vec<Certificate>, AnyError> {
    match certs(reader) {
        Ok(certs) => Ok(certs),
        Err(_) => Err("Invalid certificate chain file.".into()),
    }
}

fn get_private_key(reader: &mut BufReader<File>) -> Result<PrivateKey, AnyError> {
    match pkcs8_private_keys(reader) {
        Ok(mut keys) => {
            if keys.len() > 0 {
                Ok(keys.remove(0))
            } else {
                Err("No private key found in file.".into())
            }
        }
        Err(_) => Err("Invalid private key file.".into()),
    }
}

fn get_file_reader(file: &str) -> Result<BufReader<File>, AnyError> {
    match File::open(file) {
        Ok(file) => Ok(BufReader::new(file)),
        Err(_) => Err(format!("Can't open {:?}.", file).into()),
    }
}
