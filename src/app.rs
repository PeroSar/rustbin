use std::{collections::HashMap, net::SocketAddr, num::NonZeroUsize, path::Path, str::FromStr, sync::Arc, time::Duration};

use lru::LruCache;
use parking_lot::Mutex;
use sqlx::{
    SqlitePool,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous},
};
use sublime_color_scheme::parse_color_scheme_file;
use syntect::parsing::SyntaxSet;
use time::OffsetDateTime;
use tracing::{error, info};

use crate::{
    db::migrate_db,
    routes::app_router,
    state::AppState,
};

pub struct Config {
    pub database_url: String,
    pub host: String,
    pub port: u16,
    pub max_paste_size: usize,
    pub render_cache_capacity: NonZeroUsize,
    pub cleanup_interval: u64,
    pub db_min_connections: u32,
    pub db_max_connections: u32,
}

impl Config {
    pub fn from_env() -> Result<Self, String> {
        let database_url =
            std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite://rustbin.db".into());
        let host = std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".into());
        let port = parse_env("PORT", 3000)?;
        let max_paste_size = match std::env::var("MAX_PASTE_SIZE") {
            Ok(value) => parse_byte_size(&value)
                .ok_or_else(|| format!("invalid value for MAX_PASTE_SIZE: {value}"))?,
            Err(_) => 2 * 1024 * 1024,
        };
        let render_cache_capacity = parse_env("RENDER_CACHE_CAPACITY", 128)?;
        let render_cache_capacity = NonZeroUsize::new(render_cache_capacity)
            .ok_or("RENDER_CACHE_CAPACITY must be non-zero")?;
        let cleanup_interval = parse_env("CLEANUP_INTERVAL", 3600)?;
        let db_min_connections = parse_env("DB_MIN_CONNECTIONS", 1)?;
        let db_max_connections = parse_env("DB_MAX_CONNECTIONS", 5)?;

        Ok(Self {
            database_url,
            host,
            port,
            max_paste_size,
            render_cache_capacity,
            cleanup_interval,
            db_min_connections,
            db_max_connections,
        })
    }
}

fn parse_env<T: std::str::FromStr>(name: &str, default: T) -> Result<T, String> {
    match std::env::var(name) {
        Ok(value) => value
            .parse()
            .map_err(|_| format!("invalid value for {name}: {value}")),
        Err(_) => Ok(default),
    }
}

fn parse_byte_size(s: &str) -> Option<usize> {
    let s = s.trim();
    let (num, multiplier) = if let Some(n) = s.strip_suffix("GB") {
        (n.trim(), 1024 * 1024 * 1024)
    } else if let Some(n) = s.strip_suffix("MB") {
        (n.trim(), 1024 * 1024)
    } else if let Some(n) = s.strip_suffix("KB") {
        (n.trim(), 1024)
    } else if let Some(n) = s.strip_suffix("B") {
        (n.trim(), 1)
    } else {
        (s, 1)
    };
    num.parse::<usize>().ok()?.checked_mul(multiplier)
}

pub async fn run() -> Result<(), String> {
    let config = Config::from_env()?;

    let db = connect_db(&config).await?;
    migrate_db(&db)
        .await
        .map_err(|error| format!("database migration failed: {error}"))?;

    let syntax_set = Arc::new(SyntaxSet::load_defaults_newlines());
    let syntax_index_by_token = Arc::new(build_syntax_index_map(syntax_set.as_ref()));
    let theme = parse_color_scheme_file(Path::new("theme/gh-dark.sublime-color-scheme"))
        .map_err(|error| format!("failed to parse theme/gh-dark.sublime-color-scheme: {error}"))
        .and_then(|scheme| {
            scheme.try_into().map_err(|error| {
                format!("failed to convert theme/gh-dark.sublime-color-scheme: {error}")
            })
        })?;

    let state = Arc::new(AppState {
        db,
        syntax_set,
        syntax_index_by_token,
        render_cache: Arc::new(Mutex::new(LruCache::new(config.render_cache_capacity))),
        theme: Arc::new(theme),
    });

    tokio::spawn(cleanup_expired_pastes(
        state.db.clone(),
        config.cleanup_interval,
    ));

    let app = app_router(state, config.max_paste_size);

    let address: SocketAddr = format!("{}:{}", config.host, config.port)
        .parse()
        .map_err(|error| format!("invalid bind address {}:{}: {error}", config.host, config.port))?;
    let listener = tokio::net::TcpListener::bind(address)
        .await
        .map_err(|error| format!("failed to bind to {address}: {error}"))?;

    info!("rustbin listening on http://{address}");
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .map_err(|error| format!("server error: {error}"))?;

    Ok(())
}

pub fn init_tracing() {
    let filter =
        std::env::var("RUST_LOG").unwrap_or_else(|_| "rustbin=info,tower_http=info".into());
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .compact()
        .init();
}

async fn connect_db(config: &Config) -> Result<SqlitePool, String> {
    let connect_options = SqliteConnectOptions::from_str(&config.database_url)
        .map_err(|error| format!("invalid DATABASE_URL {}: {error}", config.database_url))?
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal)
        .synchronous(SqliteSynchronous::Normal)
        .pragma("temp_store", "MEMORY");

    SqlitePoolOptions::new()
        .min_connections(config.db_min_connections)
        .max_connections(config.db_max_connections)
        .connect_with(connect_options)
        .await
        .map_err(|error| format!("database connection failed: {error}"))
}

fn build_syntax_index_map(syntax_set: &SyntaxSet) -> HashMap<String, usize> {
    let mut mapping = HashMap::new();

    for (index, syntax) in syntax_set.syntaxes().iter().enumerate() {
        mapping.insert(syntax.name.to_ascii_lowercase(), index);
        for extension in &syntax.file_extensions {
            mapping.insert(extension.to_ascii_lowercase(), index);
        }
    }

    mapping
}

async fn cleanup_expired_pastes(db: SqlitePool, interval_secs: u64) {
    let mut interval = tokio::time::interval(Duration::from_secs(interval_secs));
    interval.tick().await;

    loop {
        interval.tick().await;
        let now = OffsetDateTime::now_utc().unix_timestamp();
        match sqlx::query!(
            "DELETE FROM pastes WHERE expires_at IS NOT NULL AND expires_at <= ?1",
            now
        )
        .execute(&db)
        .await
        {
            Ok(result) if result.rows_affected() > 0 => {
                info!(deleted = result.rows_affected(), "cleaned up expired pastes");
            }
            Err(error) => {
                error!("expired paste cleanup failed: {error}");
            }
            _ => {}
        }
    }
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to listen for ctrl+c");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to listen for SIGTERM")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {},
        () = terminate => {},
    }

    info!("shutdown signal received, stopping server");
}
