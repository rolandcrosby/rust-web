use tokio_postgres::{Error};
use bb8_postgres::PostgresConnectionManager;
use native_tls::{Certificate, TlsConnector};
use tokio_postgres_native_tls::MakeTlsConnector;
use std::fs;

type Pool = bb8::Pool<PostgresConnectionManager<native_tls::TlsConnector>>;
struct PostgresConfig {
    pg_url: String,
    ca_cert_path: String,
    client_cert_path: String
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let config = config_from_environment();
    let pool = postgres_connection_pool(&config).await?;
    println!("Ensuring DB tables exist...");
    prepare_tables(pool.clone()).await.expect("error creating DB tables!");
    println!("Starting web server");

    Ok(())
}


async fn postgres_connection_pool(cfg: &PostgresConfig) -> Option<Pool> {
    let cert = fs::read(cfg.ca_cert_path)?;
    let cert = Certificate::from_pem(&cert)?;
    let connector = TlsConnector::builder()
        .add_root_certificate(cert)
        .build()?;
    let connector = MakeTlsConnector::new(connector);
    let connection_manager = PostgresConnectionManager::new_from_stringlike(
        cfg.pg_url,
        connector,
    )
    .ok()
    .unwrap();
    bb8::Pool::builder()
        .build(connection_manager)
        .await
        .ok()?
}


fn config_from_environment() -> Option<PostgresConfig> {
    let pg_url = None;
    let ca_cert_path = None;
    let client_cert_path = None;
    for (k, v) in std::env::vars() {
        match k.as_str() {
            "PG_URL" => pg_url = Some(v),
            "CA_CERT_PATH" => ca_cert_path = Some(v),
            "CLIENT_CERT_PATH" => client_cert_path = Some(v),
            _ => {}
        }
    }
    Some(PostgresConfig { pg_url: pg_url?, ca_cert_path: ca_cert_path?, client_cert_path: client_cert_path? })
}

async fn prepare_tables(pool: Pool<???>) -> Result<String, bb8::RunError<Error>> {
    pool.run(move |client| {
        async {
            match client.query("SELECT to_regclass('sl_users')::varchar", &[]).await {
                Ok(res) => {
                    match res.len::<usize>() {
                        0 => {
                            match client.query(
                                "CREATE TABLE sl_users (
                                    id serial primary key,
                                    email varchar,
                                    password varchar,
                                    display_name varchar,
                                    time_zone varchar,
                                    created_at timestamp
                                )",
                                &[],    
                            ).await {
                                Ok(_) => Ok(("table created".to_string(), client)),
                                Err(e) => Err((e, client))
                            }
                        }
                        _ => Ok(("table already exists".to_string(), client)),
                    }
                }
                Err(e) => Err((e, client))
            }
        }
    }).await
}
