use std::env;
use sqlx::{Pool, Postgres};

pub async fn pool() -> anyhow::Result<Pool<Postgres>> {
    let pg_conn_uri = env::var("DATABASE_URL").map_err(|_| anyhow::Error::msg("DATABASE_URL environment variable not found"))?;
    let pool = sqlx::PgPool::connect(&pg_conn_uri).await?;

    Ok(pool)
}