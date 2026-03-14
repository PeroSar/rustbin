mod app;
mod constants;
mod db;
mod enry_ffi;
mod error;
mod extractors;
mod handlers;
mod highlighter;
mod preview;
mod render;
mod response;
mod routes;
mod state;

use tracing::error;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    app::init_tracing();

    if let Err(error) = app::run().await {
        error!("{error}");
        std::process::exit(1);
    }
}
