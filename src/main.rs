use std::ops::Deref;

use axum::{
    body::Bytes,
    extract::{Path, State},
    http::StatusCode,
    routing::get,
    Router,
};

use redis::{cluster::ClusterClient, AsyncCommands};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "example_tokio_redis=debug".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let env_variables = env_file_reader::read_file(".env").expect("Couldn't open .env file");
    let redis_host_var = env_variables
        .get("REDIS_HOST")
        .expect("Couldn't find REDIS_HOST variable on .env file");
    tracing::debug!("connecting to redis nodes at {}", redis_host_var);

    let redis_hosts = redis_host_var.split(';').map(|s| s.trim());
    let cluster_client = match ClusterClient::new(redis_hosts) {
        Ok(cc) => cc,
        Err(error) => panic!("Couldn't connect to redis hosts at {redis_host_var}: {error}"),
    };

    {
        let mut conn = match cluster_client.get_async_connection().await {
            Ok(c) => c,
            Err(error) => panic!("Couldn't connect to redis for pinging: {error}"),
        };

        let s: String = match conn.set("Hello", "World").await {
            Ok(s) => s,
            Err(error) => panic!("Couldn't set key-value \"Hello: World\" in redis: {error}"),
        };

        tracing::debug!("set hey \"Hello\" to value \"World\" in redis: {s}");
    }

    tracing::debug!("successfully connected to redis and pinged it");

    // build our application with some routes
    let app = Router::new().route("/:key", get(get_key).post(set_key)).with_state(cluster_client);

    // run it
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    tracing::debug!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}

type ConnectionPool = ClusterClient;

async fn get_key(Path(key): Path<String>, State(pool): State<ConnectionPool>) -> Result<String, (StatusCode, String)> {
    let mut conn = pool.get_async_connection().await.map_err(internal_error)?;
    let result: String = match conn.get(key).await {
        Ok(s) => s,
        Err(err) if err.kind() == redis::ErrorKind::TypeError => return Err((StatusCode::NOT_FOUND, String::new())),
        Err(err) => return Err(internal_error(err)),
    };

    Ok(result)
}

async fn set_key(Path(key): Path<String>, State(pool): State<ConnectionPool>, bytes: Bytes) -> Result<String, (StatusCode, String)> {
    let mut conn = pool.get_async_connection().await.map_err(internal_error)?;
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
