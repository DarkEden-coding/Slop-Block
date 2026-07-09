use serde_json::Value;

use crate::connection::{PgPool, Result};
use crate::models::{DashboardOauthToken, GithubInstallation, GithubRepository, InstallationAdmin};

pub async fn upsert_installation(
    pool: &PgPool,
    installation_id: i64,
    account_login: &str,
    account_id: Option<i64>,
    account_type: Option<&str>,
    raw: Value,
) -> Result<GithubInstallation> {
    sqlx::query_as::<_, GithubInstallation>("INSERT INTO github_installations (installation_id, account_login, account_id, account_type, raw) VALUES ($1,$2,$3,$4,$5) ON CONFLICT (installation_id) DO UPDATE SET account_login=EXCLUDED.account_login, account_id=EXCLUDED.account_id, account_type=EXCLUDED.account_type, raw=EXCLUDED.raw, deleted_at=NULL, suspended_at=NULL, updated_at=now() RETURNING *")
        .bind(installation_id).bind(account_login).bind(account_id).bind(account_type).bind(raw).fetch_one(pool).await
}

#[allow(clippy::too_many_arguments)]
pub async fn upsert_repository(
    pool: &PgPool,
    repository_id: i64,
    installation_id: i64,
    owner: &str,
    name: &str,
    full_name: &str,
    private: bool,
    default_branch: Option<&str>,
    raw: Value,
) -> Result<GithubRepository> {
    sqlx::query_as::<_, GithubRepository>("INSERT INTO github_repositories (repository_id, installation_id, owner, name, full_name, private, default_branch, raw, active) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,true) ON CONFLICT (repository_id) DO UPDATE SET installation_id=EXCLUDED.installation_id, owner=EXCLUDED.owner, name=EXCLUDED.name, full_name=EXCLUDED.full_name, private=EXCLUDED.private, default_branch=EXCLUDED.default_branch, raw=EXCLUDED.raw, active=true, updated_at=now() RETURNING *")
        .bind(repository_id).bind(installation_id).bind(owner).bind(name).bind(full_name).bind(private).bind(default_branch).bind(raw).fetch_one(pool).await
}

pub async fn get_repository(pool: &PgPool, repository_id: i64) -> Result<Option<GithubRepository>> {
    sqlx::query_as::<_, GithubRepository>(
        "SELECT * FROM github_repositories WHERE repository_id=$1 AND active=true",
    )
    .bind(repository_id)
    .fetch_optional(pool)
    .await
}

pub async fn mark_installation_deleted(pool: &PgPool, installation_id: i64) -> Result<()> {
    sqlx::query("UPDATE github_installations SET deleted_at=now(), updated_at=now() WHERE installation_id=$1")
        .bind(installation_id)
        .execute(pool)
        .await?;
    sqlx::query(
        "UPDATE github_repositories SET active=false, updated_at=now() WHERE installation_id=$1",
    )
    .bind(installation_id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn mark_installation_suspended(
    pool: &PgPool,
    installation_id: i64,
    suspended: bool,
) -> Result<()> {
    let sql = if suspended {
        "UPDATE github_installations SET suspended_at=now(), updated_at=now() WHERE installation_id=$1"
    } else {
        "UPDATE github_installations SET suspended_at=NULL, updated_at=now() WHERE installation_id=$1"
    };
    sqlx::query(sql).bind(installation_id).execute(pool).await?;
    Ok(())
}

pub async fn upsert_installation_admin(
    pool: &PgPool,
    installation_id: i64,
    github_user_id: i64,
    login: &str,
) -> Result<InstallationAdmin> {
    sqlx::query_as::<_, InstallationAdmin>("INSERT INTO installation_admins (installation_id, github_user_id, login) VALUES ($1,$2,$3) ON CONFLICT (installation_id, github_user_id) DO UPDATE SET login=EXCLUDED.login, updated_at=now() RETURNING *")
        .bind(installation_id)
        .bind(github_user_id)
        .bind(login)
        .fetch_one(pool)
        .await
}

pub async fn user_can_manage_installation(
    pool: &PgPool,
    installation_id: i64,
    github_user_id: i64,
) -> Result<bool> {
    let (exists,): (bool,) = sqlx::query_as("SELECT EXISTS(SELECT 1 FROM installation_admins WHERE installation_id=$1 AND github_user_id=$2)")
        .bind(installation_id)
        .bind(github_user_id)
        .fetch_one(pool)
        .await?;
    Ok(exists)
}

pub async fn user_can_manage_repo(
    pool: &PgPool,
    repository_id: i64,
    github_user_id: i64,
) -> Result<bool> {
    let (exists,): (bool,) = sqlx::query_as("SELECT EXISTS(SELECT 1 FROM github_repositories r JOIN installation_admins a ON a.installation_id=r.installation_id WHERE r.repository_id=$1 AND r.active=true AND a.github_user_id=$2)")
        .bind(repository_id)
        .bind(github_user_id)
        .fetch_one(pool)
        .await?;
    Ok(exists)
}

pub async fn upsert_dashboard_oauth_token(
    pool: &PgPool,
    github_user_id: i64,
    login: &str,
    access_token_encrypted: &str,
) -> Result<DashboardOauthToken> {
    sqlx::query_as::<_, DashboardOauthToken>("INSERT INTO dashboard_oauth_tokens (github_user_id, login, access_token_encrypted) VALUES ($1,$2,$3) ON CONFLICT (github_user_id) DO UPDATE SET login=EXCLUDED.login, access_token_encrypted=EXCLUDED.access_token_encrypted, updated_at=now() RETURNING *")
        .bind(github_user_id)
        .bind(login)
        .bind(access_token_encrypted)
        .fetch_one(pool)
        .await
}

pub async fn get_dashboard_oauth_token(
    pool: &PgPool,
    github_user_id: i64,
) -> Result<Option<DashboardOauthToken>> {
    sqlx::query_as::<_, DashboardOauthToken>(
        "SELECT * FROM dashboard_oauth_tokens WHERE github_user_id=$1",
    )
    .bind(github_user_id)
    .fetch_optional(pool)
    .await
}

pub async fn list_repositories(
    pool: &PgPool,
    installation_id: i64,
) -> Result<Vec<GithubRepository>> {
    sqlx::query_as::<_, GithubRepository>(
        "SELECT * FROM github_repositories WHERE installation_id=$1 AND active=true ORDER BY full_name",
    )
    .bind(installation_id)
    .fetch_all(pool)
    .await
}

pub async fn list_repositories_page(
    pool: &PgPool,
    installation_id: i64,
    limit: i64,
    offset: i64,
) -> Result<Vec<GithubRepository>> {
    sqlx::query_as::<_, GithubRepository>(
        "SELECT * FROM github_repositories WHERE installation_id=$1 AND active=true ORDER BY full_name LIMIT $2 OFFSET $3",
    )
    .bind(installation_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
}

pub async fn mark_repository_inactive(pool: &PgPool, repository_id: i64) -> Result<()> {
    sqlx::query(
        "UPDATE github_repositories SET active=false, updated_at=now() WHERE repository_id=$1",
    )
    .bind(repository_id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn deactivate_repositories_not_in(
    pool: &PgPool,
    installation_id: i64,
    active_repository_ids: &[i64],
) -> Result<u64> {
    let result = sqlx::query(
        "UPDATE github_repositories
         SET active=false, updated_at=now()
         WHERE installation_id=$1
           AND active=true
           AND NOT (repository_id = ANY($2))",
    )
    .bind(installation_id)
    .bind(active_repository_ids)
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}

pub async fn list_installations_page(
    pool: &PgPool,
    limit: i64,
    offset: i64,
) -> Result<Vec<GithubInstallation>> {
    sqlx::query_as::<_, GithubInstallation>(
        "SELECT * FROM github_installations WHERE deleted_at IS NULL ORDER BY account_login LIMIT $1 OFFSET $2",
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
}

pub async fn list_user_installations_page(
    pool: &PgPool,
    github_user_id: i64,
    limit: i64,
    offset: i64,
) -> Result<Vec<GithubInstallation>> {
    sqlx::query_as::<_, GithubInstallation>(
        "SELECT i.* FROM github_installations i
         JOIN installation_admins a ON a.installation_id=i.installation_id
         WHERE a.github_user_id=$1 AND i.deleted_at IS NULL
         ORDER BY i.account_login
         LIMIT $2 OFFSET $3",
    )
    .bind(github_user_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
}

pub async fn list_repositories_for_user_page(
    pool: &PgPool,
    github_user_id: Option<i64>,
    limit: i64,
    offset: i64,
) -> Result<Vec<GithubRepository>> {
    if let Some(user_id) = github_user_id {
        sqlx::query_as::<_, GithubRepository>(
            "SELECT r.* FROM github_repositories r
             JOIN installation_admins a ON a.installation_id=r.installation_id
             WHERE a.github_user_id=$1 AND r.active=true
             ORDER BY r.full_name
             LIMIT $2 OFFSET $3",
        )
        .bind(user_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await
    } else {
        sqlx::query_as::<_, GithubRepository>(
            "SELECT * FROM github_repositories WHERE active=true ORDER BY full_name LIMIT $1 OFFSET $2",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await
    }
}
