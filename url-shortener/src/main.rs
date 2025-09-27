use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Redirect},
    routing::{get, post},
};
use rand::Rng;
use rand::distr::Alphanumeric;
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, SqlitePool, sqlite::SqlitePoolOptions};
use std::net::SocketAddr;

#[derive(Serialize, FromRow)]
struct URL {
    id: i64,
    original_url: String,
    short_url: String,
    short_code: String,
    visit_count: i64,
}

#[derive(Deserialize)]
struct NewUrl {
    original_url: String,
}

#[derive(Deserialize)]
struct ShortnerQuery {
    id: Option<i64>,
    original_url: Option<String>,
    short_url: Option<String>,
}

#[derive(Deserialize)]
struct Params {
    short_code: String,
}

#[derive(Serialize)]
struct HealthResponse {
    healthz: &'static str,
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // Create SQLite connection pool
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect("sqlite://urls.db")
        .await?;

    // Run migrations (create table if not exists)
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS urls (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            original_url TEXT NOT NULL,
            short_url TEXT NOT NULL,
            short_code TEXT NOT NULL UNIQUE,
            visit_count INTEGER NOT NULL DEFAULT 0
        )
        "#,
    )
    .execute(&pool)
    .await?;

    let app = Router::new()
        .route("/healthz", get(healthz))
        .route("/shorten", post(shorten))
        .route("/r/{short_code}", get(redirect))
        .route("/urls", get(get_url))
        .route("/stats", get(get_stats))
        .with_state(pool);

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    println!("Listening on {}", addr);
    axum::serve(tokio::net::TcpListener::bind(addr).await?, app).await?;

    Ok(())
}

async fn healthz() -> impl IntoResponse {
    Json(HealthResponse { healthz: "ok" })
}

async fn shorten(
    State(pool): State<SqlitePool>,
    Json(payload): Json<NewUrl>,
) -> Result<Json<URL>, (StatusCode, String)> {
    // generate unique short code
    let short_code = loop {
        let candidate = generate_short_code(6);
        let exists: Option<(i64,)> = sqlx::query_as("SELECT id FROM urls WHERE short_code = ?")
            .bind(&candidate)
            .fetch_optional(&pool)
            .await
            .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
        if exists.is_none() {
            break candidate;
        }
    };

    let short_url = format!("http://short.url/{}", short_code);

    let result = sqlx::query_as::<_, URL>(
        r#"
        INSERT INTO urls (original_url, short_url, short_code, visit_count)
        VALUES (?, ?, ?, 0)
        RETURNING id, original_url, short_url, short_code, visit_count
        "#,
    )
    .bind(&payload.original_url)
    .bind(&short_url)
    .bind(&short_code)
    .fetch_one(&pool)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(result))
}

async fn redirect(State(pool): State<SqlitePool>, Path(params): Path<Params>) -> impl IntoResponse {
    if let Some(url) = sqlx::query_as::<_, URL>("SELECT * FROM urls WHERE short_code = ?")
        .bind(&params.short_code)
        .fetch_optional(&pool)
        .await
        .unwrap()
    {
        // increment visit count
        let _ = sqlx::query("UPDATE urls SET visit_count = visit_count + 1 WHERE id = ?")
            .bind(url.id)
            .execute(&pool)
            .await;

        Redirect::temporary(&url.original_url).into_response()
    } else {
        (StatusCode::NOT_FOUND, "Short code not found").into_response()
    }
}

async fn get_url(
    State(pool): State<SqlitePool>,
    Query(params): Query<ShortnerQuery>,
) -> impl IntoResponse {
    let mut query = "SELECT * FROM urls WHERE 1=1".to_string();
    let mut binds: Vec<String> = Vec::new();

    if let Some(id) = params.id {
        query.push_str(" AND id = ?");
        binds.push(id.to_string());
    }
    if let Some(original_url) = params.original_url {
        query.push_str(" AND original_url = ?");
        binds.push(original_url);
    }
    if let Some(short_url) = params.short_url {
        query.push_str(" AND short_url = ?");
        binds.push(short_url);
    }

    let mut sql = sqlx::query_as::<_, URL>(&query);
    for b in binds {
        sql = sql.bind(b);
    }

    let urls = sql.fetch_all(&pool).await.unwrap_or_default();

    Json(urls)
}

async fn get_stats(
    State(pool): State<SqlitePool>,
    Query(params): Query<ShortnerQuery>,
) -> impl IntoResponse {
    get_url(State(pool), Query(params)).await
}

fn generate_short_code(len: usize) -> String {
    rand::rng()
        .sample_iter(&Alphanumeric)
        .take(len)
        .map(char::from)
        .collect()
}
