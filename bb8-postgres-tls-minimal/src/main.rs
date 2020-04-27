use bb8_postgres::PostgresConnectionManager;
use native_tls::{Certificate, Identity, TlsConnector};
use postgres_native_tls::MakeTlsConnector;
use std::fs;
use std::io;
use std::str::FromStr;
use tokio_postgres::config::Config;

#[derive(Debug)]
enum AppError {
    Io(io::Error),
    NativeTls(native_tls::Error),
    TokioPostgres(tokio_postgres::Error),
    Bb8(bb8::RunError<tokio_postgres::Error>),
}

impl From<io::Error> for AppError {
    fn from(error: io::Error) -> Self {
        AppError::Io(error)
    }
}
impl From<native_tls::Error> for AppError {
    fn from(error: native_tls::Error) -> Self {
        AppError::NativeTls(error)
    }
}
impl From<tokio_postgres::error::Error> for AppError {
    fn from(error: tokio_postgres::error::Error) -> Self {
        AppError::TokioPostgres(error)
    }
}
impl From<bb8::RunError<tokio_postgres::error::Error>> for AppError {
    fn from(error: bb8::RunError<tokio_postgres::error::Error>) -> Self {
        AppError::Bb8(error)
    }
}

async fn pg_main() -> Result<(), AppError> {
    let make_tls_connect = get_make_tls_connect()?;
    let config = get_config()?;
    let (client, connection) = config.connect(make_tls_connect).await?;
    tokio::spawn(async move {
        if let Err(_) = connection.await {
            eprintln!("TODO: figure out if there's a reason to handle any errors here")
        }
    });
    let res = client.query("SELECT 'howdy'", &[]).await?;
    println!("message from Postgres: {}", res[0].get::<usize, &str>(0));
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), AppError> {
// async fn bb8_main() -> Result<(), AppError> {
    let config = get_config()?;
    let make_tls_connect = get_make_tls_connect()?;
    let mgr = PostgresConnectionManager::new(config, make_tls_connect);
    let pool = bb8::Builder::new().build(mgr).await?;
    let _ = pool.run(|connection| async {
        match connection.query("SELECT 'howdy'", &[]).await {
            Ok(res) => {
                println!("bb8 says: {}", res[0].get::<usize, &str>(0));
                Ok(((), connection))
            }
            Err(e) => Err((e, connection))
        }
    }).await?;
    Ok(())
}

fn get_make_tls_connect() -> Result<MakeTlsConnector, AppError> {
    let cert = fs::read("../compose/certs/rootCA.pem")?;
    let cert = Certificate::from_pem(&cert)?;
    let identity = fs::read("../compose/certs/docker-client.p12")?;
    let identity = Identity::from_pkcs12(&identity, "changeit")?;
    let connector = TlsConnector::builder()
        .add_root_certificate(cert)
        .identity(identity)
        .build()?;
    Ok(MakeTlsConnector::new(connector))
}

fn get_config() -> Result<Config, tokio_postgres::Error> {
    Config::from_str(
        "postgres://docker:docker@localhost:5432/docker?sslmode=require&connect_timeout=10",
    )
}
