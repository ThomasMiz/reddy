use std::ops::Deref;

use axum::{
    body::Bytes, extract::{Path, State}, http::StatusCode, routing::get, Router
};
use bb8::Pool;
use bb8_redis::RedisConnectionManager;
use redis::AsyncCommands;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use bb8_redis::bb8;

#[tokio::main]
async fn main() {

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "example_tokio_redis=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let env_variables = env_file_reader::read_file(".env").expect("Couldn't open .env file");
    let redis_host = env_variables.get("REDIS_HOST").expect("Couldn't find REDIS_HOST variable on .env file");
    tracing::debug!("connecting to redis at {}", redis_host);
    let manager = RedisConnectionManager::new(redis_host.as_str()).unwrap();
    let pool = bb8::Pool::builder().build(manager).await.unwrap();

    /*{
        // ping the database before starting
        let mut conn = pool.get().await.unwrap();
        conn.set::<&str, &str, ()>("foo", "bar").await.unwrap();
        let result: String = conn.get("foo").await.unwrap();
        assert_eq!(result, "bar");
    }*/
    tracing::debug!("successfully connected to redis and pinged it");

    // build our application with some routes
    let app = Router::new()
        .route(
            "/:key",
            get(get_key).post(set_key),
        )
        .with_state(pool);

    // run it
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000")
        .await
        .unwrap();
    tracing::debug!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}

type ConnectionPool = Pool<RedisConnectionManager>;

async fn get_key(
    Path(key): Path<String>,
    State(pool): State<ConnectionPool>,
) -> Result<String, (StatusCode, String)> {
    let mut conn = pool.get().await.map_err(internal_error)?;
    let result: String = conn.get(key).await.map_err(internal_error)?;
    Ok(result)
}

async fn set_key(
    Path(key): Path<String>,
    State(pool): State<ConnectionPool>,
    bytes: Bytes,
) -> Result<String, (StatusCode, String)> {
    let mut conn = pool.get().await.map_err(internal_error)?;
    let result: String = conn.set(key, bytes.deref()).await.map_err(internal_error)?;
    Ok(result)
}

/// Utility function for mapping any error into a `500 Internal Server Error` response.
fn internal_error<E>(err: E) -> (StatusCode, String)
where
    E: std::error::Error,
{
    (StatusCode::INTERNAL_SERVER_ERROR, err.to_string())
}