use windmill_common::{
    error::{self, Error},
    get_database_url, DatabaseUrl,
};

// Single source of truth in windmill_common so the DB-health sizing guidance
// (windmill-api/src/db_health.rs) and the actual pool sizing here can't drift.
pub use windmill_common::{
    DEFAULT_MAX_CONNECTIONS_INDEXER, DEFAULT_MAX_CONNECTIONS_SERVER, DEFAULT_MAX_CONNECTIONS_WORKER,
};
#[cfg(feature = "operator")]
pub const DEFAULT_MAX_CONNECTIONS_OPERATOR: u32 = 2;

pub async fn initial_connection() -> Result<sqlx::Pool<sqlx::Postgres>, error::Error> {
    let connect_options = get_database_url().await?.connect_options().await?;
    sqlx::postgres::PgPoolOptions::new()
        .max_connections(2)
        .connect_with(connect_options)
        .await
        .map_err(|err| Error::ConnectingToDatabase(err.to_string()))
}

/// Connect to the database for the Kubernetes operator process.
///
/// Long-running operator pods need IAM RDS / Entra ID token refresh just like the server,
/// otherwise new pool connections start failing once the initial token expires (~15 min).
#[cfg(feature = "operator")]
pub async fn operator_connection(
) -> anyhow::Result<sqlx::Pool<sqlx::Postgres>> {
    let database_url = get_database_url().await?;
    let pool = connect(
        database_url.clone(),
        DEFAULT_MAX_CONNECTIONS_OPERATOR,
        false,
    )
    .await?;

    Ok(pool)
}

pub async fn connect_db(
    server_mode: bool,
    indexer_mode: bool,
    worker_mode: bool,
    num_workers: i32,
) -> anyhow::Result<sqlx::Pool<sqlx::Postgres>> {
    use anyhow::Context;

    let database_url = get_database_url().await?;

    let max_connections = match std::env::var("DATABASE_CONNECTIONS") {
        Ok(n) => n.parse::<u32>().context("invalid DATABASE_CONNECTIONS")?,
        Err(_) => {
            if server_mode {
                DEFAULT_MAX_CONNECTIONS_SERVER
            } else if indexer_mode {
                DEFAULT_MAX_CONNECTIONS_INDEXER
            } else {
                DEFAULT_MAX_CONNECTIONS_WORKER + (num_workers.max(1) as u32) - 1
            }
        }
    };

    let pool = connect(database_url.clone(), max_connections, worker_mode).await?;

    Ok(pool)
}

pub async fn connect(
    database_url: DatabaseUrl,
    max_connections: u32,
    worker_mode: bool,
) -> Result<sqlx::Pool<sqlx::Postgres>, error::Error> {
    use sqlx::Executor;
    use std::time::Duration;
    let mut pool_options = sqlx::postgres::PgPoolOptions::new()
        .min_connections(0)
        .max_connections(max_connections)
        .max_lifetime(Duration::from_secs(30 * 60)); // 30 mins
    if worker_mode {
        pool_options = pool_options.idle_timeout(Duration::from_secs(60));
    }
    pool_options
        .before_acquire(|_, meta| {
            Box::pin(async move { Ok(!windmill_common::db::connection_is_retired(meta.age)) })
        })
        .after_connect(move |conn, _| {
            if worker_mode {
                Box::pin(async move {
                    if let Err(e) = conn
                        .execute(
                            r#"
        SET enable_seqscan = OFF;
        SET statement_timeout = '5min';
        SET idle_in_transaction_session_timeout = '10min';
        SET tcp_keepalives_idle = 300;
        SET tcp_keepalives_interval = 60;
        SET tcp_keepalives_count = 10;"#,
                        )
                        .await
                    {
                        tracing::error!("Error setting postgres settings: {}", e);
                    }
                    Ok(())
                })
            } else {
                Box::pin(async move {
                    if let Err(e) = conn
                        .execute(
                            r#"
        SET statement_timeout = '5min';
        SET idle_in_transaction_session_timeout = '10min';
        SET tcp_keepalives_idle = 300;
        SET tcp_keepalives_interval = 60;
        SET tcp_keepalives_count = 10;"#,
                        )
                        .await
                    {
                        tracing::error!("Error setting postgres settings: {}", e);
                    }
                    Ok(())
                })
            }
        })
        .connect_with(
            database_url
                .connect_options()
                .await?
                .statement_cache_capacity(400),
        )
        .await
        .map_err(|err| Error::ConnectingToDatabase(err.to_string()))
}
