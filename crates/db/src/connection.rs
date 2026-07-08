use sqlx::postgres::PgPoolOptions;

pub type PgPool = sqlx::PgPool;
pub type Result<T> = std::result::Result<T, sqlx::Error>;

pub async fn connect(database_url: &str) -> Result<PgPool> {
    PgPoolOptions::new()
        .max_connections(10)
        .connect(database_url)
        .await
}

pub async fn migrate(pool: &PgPool) -> Result<()> {
    sqlx::migrate!("../../migrations").run(pool).await?;
    Ok(())
}
