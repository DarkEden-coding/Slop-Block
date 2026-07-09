use std::time::Duration;

use sqlx::postgres::PgPoolOptions;

pub type PgPool = sqlx::PgPool;
pub type Result<T> = std::result::Result<T, sqlx::Error>;

pub async fn connect(database_url: &str) -> Result<PgPool> {
    connect_with_options(database_url, 20, 30).await
}

pub async fn connect_with_options(
    database_url: &str,
    max_connections: u32,
    acquire_timeout_secs: u64,
) -> Result<PgPool> {
    PgPoolOptions::new()
        .max_connections(max_connections.max(1))
        .acquire_timeout(Duration::from_secs(acquire_timeout_secs.max(1)))
        .connect(database_url)
        .await
}

pub async fn migrate(pool: &PgPool) -> Result<()> {
    sqlx::migrate!("../../migrations").run(pool).await?;
    Ok(())
}
