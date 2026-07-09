use crate::connection::{PgPool, Result};

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ObservedGithubSubject {
    pub repository_id: i64,
    pub subject_type: String,
    pub subject_id: String,
    pub github_user_id: i64,
    pub login: String,
    pub html_url: String,
    pub head_sha: Option<String>,
}

#[allow(clippy::too_many_arguments)]
pub async fn upsert_observed_github_subject(
    pool: &PgPool,
    repository_id: i64,
    subject_type: &str,
    subject_id: &str,
    github_user_id: i64,
    login: &str,
    html_url: &str,
    head_sha: Option<&str>,
) -> Result<()> {
    sqlx::query("INSERT INTO observed_github_subjects (repository_id,subject_type,subject_id,github_user_id,login,html_url,head_sha) VALUES ($1,$2,$3,$4,$5,$6,$7) ON CONFLICT (repository_id,subject_type,subject_id) DO UPDATE SET github_user_id=EXCLUDED.github_user_id, login=EXCLUDED.login, html_url=EXCLUDED.html_url, head_sha=EXCLUDED.head_sha, updated_at=now()")
        .bind(repository_id).bind(subject_type).bind(subject_id).bind(github_user_id).bind(login).bind(html_url).bind(head_sha).execute(pool).await?;
    Ok(())
}

pub async fn observed_subjects_for_user(
    pool: &PgPool,
    repository_id: i64,
    github_user_id: i64,
) -> Result<Vec<ObservedGithubSubject>> {
    sqlx::query_as("SELECT repository_id,subject_type,subject_id,github_user_id,login,html_url,head_sha FROM observed_github_subjects WHERE repository_id=$1 AND github_user_id=$2 ORDER BY updated_at DESC")
        .bind(repository_id).bind(github_user_id).fetch_all(pool).await
}
