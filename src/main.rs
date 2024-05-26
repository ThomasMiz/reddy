use std::ops::Deref;

use axum::{
    body::Bytes,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Router,
};

use redis::AsyncCommands;
use tracing_subscriber::{filter, layer::SubscriberExt, util::SubscriberInitExt, Layer};

fn get_key_instance_index(key: &str, redis_instance_count: usize) -> usize {
    return key.bytes().map(|b| b as usize).sum::<usize>() % redis_instance_count;
}

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().with_filter(filter::LevelFilter::DEBUG))
        .init();

    let env_variables = env_file_reader::read_file(".env").expect("Couldn't open .env file");
    let redis_host_var = env_variables
        .get("REDIS_HOSTS")
        .expect("Couldn't find REDIS_HOSTS variable on .env file");
    tracing::debug!("Connecting to Redis nodes at {}", redis_host_var);

    let mut clients = Vec::new();
    for host in redis_host_var.split(';').map(|s| s.trim()) {
        let client = match redis::Client::open(host) {
            Ok(h) => h,
            Err(err) => panic!("Invalid Redis host format on \"{host}\": {err}"),
        };

        let mut conn = match client.get_multiplexed_tokio_connection().await {
            Ok(c) => c,
            Err(err) => {
                tracing::error!("Could not connect to Redis instance at {host}: {err}");
                continue;
            }
        };

        tracing::debug!("Setting test key on host {host}");
        let _: String = match conn.set("test", "test").await {
            Ok(s) => s,
            Err(err) => {
                tracing::error!("Could not SET test key on redis host {host}: {err}");
                continue;
            }
        };

        tracing::debug!("Getting test key on host {host}");
        let _: String = match conn.get("test").await {
            Ok(s) => s,
            Err(err) => {
                tracing::error!("Could SET but not GET test key on redis host {host}: {err}");
                continue;
            }
        };

        tracing::info!("Redis host {host} working properly");
        clients.push(client);
    }

    // build our application with some routes
    let app = Router::new().route("/:key", get(get_key).post(set_key)).with_state(clients);

    // run it
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    tracing::debug!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}

type ConnectionPool = Vec<redis::Client>;

async fn get_key(Path(key): Path<String>, State(pool): State<ConnectionPool>) -> impl IntoResponse {
    let client = &pool[get_key_instance_index(&key, pool.len())];

    let mut conn = client
        .get_multiplexed_async_connection()
        .await
        .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, format!("Could not connect to Redis: {err}")))?;

    let result: String = match conn.get(key).await {
        Ok(s) => s,
        Err(err) if err.kind() == redis::ErrorKind::TypeError => return Err((StatusCode::NOT_FOUND, String::new())),
        Err(err) => return Err(internal_error(err)),
    };

    Ok(result)
}

async fn set_key(Path(key): Path<String>, State(pool): State<ConnectionPool>, bytes: Bytes) -> Result<String, (StatusCode, String)> {
    let client = &pool[get_key_instance_index(&key, pool.len())];

    let mut conn = client
        .get_multiplexed_async_connection()
        .await
        .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, format!("Could not connect to Redis: {err}")))?;

    let result: String = match conn.set(key, bytes.deref()).await {
        Ok(s) => s,
        Err(err) if err.kind() == redis::ErrorKind::TypeError => return Err((StatusCode::NOT_FOUND, String::new())),
        Err(err) => return Err(internal_error(err)),
    };

    Ok(result)
}

/// Utility function for mapping any error into a `500 Internal Server Error` response.
fn internal_error<E>(err: E) -> (StatusCode, String)
where
    E: std::error::Error,
{
    (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
}
