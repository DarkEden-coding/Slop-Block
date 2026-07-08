use time::OffsetDateTime;

use crate::connection::{PgPool, Result};

pub async fn upsert_github_rate_limit(
    pool: &PgPool,
    bucket: &str,
    remaining: Option<i32>,
    reset_at: Option<OffsetDateTime>,
    paused_until: Option<OffsetDateTime>,
    status: Option<i32>,
    error: Option<&str>,
) -> Result<()> {
    sqlx::query("INSERT INTO github_rate_limits (bucket,remaining,reset_at,paused_until,last_status,last_error) VALUES ($1,$2,$3,$4,$5,$6) ON CONFLICT (bucket) DO UPDATE SET remaining=EXCLUDED.remaining, reset_at=EXCLUDED.reset_at, paused_until=EXCLUDED.paused_until, last_status=EXCLUDED.last_status, last_error=EXCLUDED.last_error, updated_at=now()")
        .bind(bucket).bind(remaining).bind(reset_at).bind(paused_until).bind(status).bind(error).execute(pool).await?;
    Ok(())
}

pub async fn github_pause_until(pool: &PgPool, bucket: &str) -> Result<Option<OffsetDateTime>> {
    let row: Option<(Option<OffsetDateTime>,)> = sqlx::query_as(
        "SELECT paused_until FROM github_rate_limits WHERE bucket=$1 AND paused_until > now()",
    )
    .bind(bucket)
    .fetch_optional(pool)
    .await?;
    Ok(row.and_then(|(x,)| x))
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
pub struct GithubRateLimitPause {
    pub bucket: String,
    pub remaining: Option<i32>,
    pub reset_at: Option<OffsetDateTime>,
    pub paused_until: OffsetDateTime,
    pub last_status: Option<i32>,
    pub last_error: Option<String>,
}

pub async fn list_active_rate_limit_pauses_for_installation(
    pool: &PgPool,
    installation_id: i64,
) -> Result<Vec<GithubRateLimitPause>> {
    let pattern = format!("installation:{installation_id}:%");
    sqlx::query_as::<_, GithubRateLimitPause>(
        "SELECT bucket, remaining, reset_at, paused_until, last_status, last_error
         FROM github_rate_limits
         WHERE bucket LIKE $1 AND paused_until > now()
         ORDER BY paused_until DESC",
    )
    .bind(pattern)
    .fetch_all(pool)
    .await
}
