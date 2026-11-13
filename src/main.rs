mod cli;
mod handlers;
mod paths;

use axum::{
    Router,
    extract::DefaultBodyLimit,
    routing::{get, post},
};
use clap::Parser;
use cli::Cli;
use handlers::{file_info, list_files, upload_files};
use local_ip_address::local_ip;
use qrcode::{QrCode, render::unicode};
use std::net::SocketAddr;
use std::path::PathBuf;

fn build_router(root: PathBuf) -> Router {
    Router::new()
        .route("/", get(list_files))
        .route("/files/{*filename}", get(file_info))
        .route("/browse/{*directory}", get(list_files))
        .route("/upload", post(upload_files))
        .route("/upload/{*directory}", post(upload_files))
        .layer(DefaultBodyLimit::disable())
        .with_state(root)
}

fn print_qr_code(port: u16) -> std::io::Result<()> {
    let ip = local_ip().map_err(std::io::Error::other)?;
    let url = format!("http://{ip}:{port}");

    let code = QrCode::new(url.as_bytes()).map_err(std::io::Error::other)?;

    let image = code
        .render::<unicode::Dense1x2>()
        .dark_color(unicode::Dense1x2::Light)
        .light_color(unicode::Dense1x2::Dark)
        .build();

    println!("Open on your phone: {url}");
    println!("{image}");

    Ok(())
}

async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("Failed to listen for Ctrl+C");

    println!("Shutting down server...");
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let cli = Cli::parse();
    let root = cli.validate_and_resolve_root()?;
    let app = build_router(root);

    let ip = if cli.lan {
        [0, 0, 0, 0]
    } else {
        [127, 0, 0, 1]
    };
    let address = SocketAddr::from((ip, cli.port));
    let listener = tokio::net::TcpListener::bind(address).await?;
    println!("Listening on http://{}", listener.local_addr()?);

    if cli.qr {
        print_qr_code(cli.port)?;
    }

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    println!("Server stopped.");

    Ok(())
}
