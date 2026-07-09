use api::{router, AppState, Config};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing();

    let config = Config::from_env()?;
    let addr = config.addr()?;

    tracing::info!("connecting to database");
    let pool = db::connect_with_options(
        &config.database_url,
        config.database_max_connections,
        config.database_acquire_timeout_secs,
    )
    .await?;

    tracing::info!("running database migrations");
    db::migrate(&pool).await?;

    let state = AppState::new(config, pool.clone());
    let job_runners = api::job_runner::spawn_job_runner(state.clone(), pool.clone());
    let app = router(state);
    let listener = tokio::net::TcpListener::bind(addr).await?;

    tracing::info!(%addr, "starting API server");
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await?;
    for job_runner in job_runners {
        job_runner.abort();
    }

    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        if let Err(error) = tokio::signal::ctrl_c().await {
            tracing::error!(%error, "failed to install Ctrl+C handler");
        }
    };

    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut signal) => {
                signal.recv().await;
            }
            Err(error) => tracing::error!(%error, "failed to install terminate handler"),
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    tracing::info!("shutdown signal received");
}

fn init_tracing() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "api=info,tower_http=info,sqlx=warn".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
}
