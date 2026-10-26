use askama::Template;
use axum::{
    Router,
    body::Body,
    extract::{Path, State},
    http::{StatusCode, header},
    response::{Html, IntoResponse},
    routing::get,
};
use clap::Parser;
use local_ip_address::local_ip;
use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};
use qrcode::{QrCode, render::unicode};
use std::net::SocketAddr;
use std::path::PathBuf;
use tokio_util::io::ReaderStream;

#[derive(Parser, Debug)]
#[command(version, about = "Share local files through an HTTP server")]
struct Cli {
    /// Directory to share (default: ~/Desktop/local_server)
    directory: Option<PathBuf>,

    /// Port on which the server listens
    #[arg(short, long, default_value_t = 8000)]
    port: u16,

    /// Allow other devices to connect (no authentication)
    #[arg(long)]
    lan: bool,

    /// Display a QR code for phone access (requires --lan)
    #[arg(long)]
    qr: bool,
}

struct FileEntry {
    name: String,
    href: String,
    kind: &'static str,
    is_clickable: bool,
    is_directory: bool,
}

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate {
    entries: Vec<FileEntry>,
    parent_href: Option<String>,
}

fn is_valid_relative_path(path: &str) -> bool {
    !path.contains('\\')
        && path
            .split('/')
            .all(|part| !part.is_empty() && !part.starts_with('.'))
}

async fn file_info(
    State(root): State<PathBuf>,
    Path(filename): Path<String>,
) -> Result<impl IntoResponse, StatusCode> {
    if !is_valid_relative_path(&filename) {
        return Err(StatusCode::BAD_REQUEST);
    }

    let file_path: PathBuf = tokio::fs::canonicalize(root.join(&filename))
        .await
        .map_err(io_error_to_status)?;

    if !file_path.starts_with(&root) {
        return Err(StatusCode::FORBIDDEN);
    }

    let metadata = tokio::fs::metadata(&file_path)
        .await
        .map_err(io_error_to_status)?;

    if !metadata.is_file() {
        return Err(StatusCode::NOT_FOUND);
    }

    let file = tokio::fs::File::open(&file_path)
        .await
        .map_err(io_error_to_status)?;
    let stream = ReaderStream::new(file);

    let content_type = mime_guess::from_path(&file_path)
        .first_or_octet_stream()
        .to_string();

    Ok((
        [
            (header::CONTENT_TYPE, content_type),
            (header::CONTENT_DISPOSITION, "attachment".to_string()),
        ],
        Body::from_stream(stream),
    ))
}

async fn list_files(
    State(root): State<PathBuf>,
    directory: Option<Path<String>>,
) -> Result<Html<String>, StatusCode> {
    let relative = directory.map(|Path(value)| value).unwrap_or_default();

    if !relative.is_empty() && !is_valid_relative_path(&relative) {
        return Err(StatusCode::BAD_REQUEST);
    }

    let current = tokio::fs::canonicalize(root.join(&relative))
        .await
        .map_err(io_error_to_status)?;

    if !current.starts_with(&root) {
        return Err(StatusCode::FORBIDDEN);
    }

    if !current.is_dir() {
        return Err(StatusCode::NOT_FOUND);
    }

    let mut directory = tokio::fs::read_dir(&current)
        .await
        .map_err(io_error_to_status)?;

    let mut entries = Vec::new();

    while let Some(entry) = directory.next_entry().await.map_err(io_error_to_status)? {
        let name = entry.file_name().to_string_lossy().into_owned();

        if name.starts_with('.') {
            continue;
        }

        let file_type = entry.file_type().await.map_err(io_error_to_status)?;

        let kind = if file_type.is_file() {
            "FILE"
        } else if file_type.is_dir() {
            "DIR"
        } else if file_type.is_symlink() {
            "LINK"
        } else {
            "OTHER"
        };

        let encoded_path = relative
            .split('/')
            .filter(|part| !part.is_empty())
            .chain(std::iter::once(name.as_str()))
            .map(|part| utf8_percent_encode(part, NON_ALPHANUMERIC).to_string())
            .collect::<Vec<_>>()
            .join("/");

        let href = if file_type.is_dir() {
            format!("/browse/{encoded_path}")
        } else {
            format!("/files/{encoded_path}")
        };

        entries.push(FileEntry {
            name,
            href,
            kind,
            is_clickable: file_type.is_file() || file_type.is_dir(),
            is_directory: file_type.is_dir(),
        });
    }

    entries.sort_by(|a, b| {
        b.is_directory
            .cmp(&a.is_directory)
            .then_with(|| a.name.cmp(&b.name))
    });

    let parent_href = if relative.is_empty() {
        None
    } else {
        let parent = relative.rsplit_once('/').map(|(path, _)| path);

        Some(match parent {
            Some(path) => {
                let encoded = path
                    .split('/')
                    .map(|part| utf8_percent_encode(part, NON_ALPHANUMERIC).to_string())
                    .collect::<Vec<_>>()
                    .join("/");

                format!("/browse/{encoded}")
            }
            None => "/".to_string(),
        })
    };

    let html = IndexTemplate {
        entries,
        parent_href,
    }
    .render()
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Html(html))
}

async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("Failed to listen for Ctrl+C");

    println!("Shutting down server...");
}

fn io_error_to_status(error: std::io::Error) -> StatusCode {
    match error.kind() {
        std::io::ErrorKind::NotFound => StatusCode::NOT_FOUND,
        std::io::ErrorKind::PermissionDenied => StatusCode::FORBIDDEN,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let cli = Cli::parse();
    if cli.qr && !cli.lan {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "--qr requires --lan",
        ));
    }

    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| std::io::Error::other("HOME is not set"))?;

    let directory = match cli.directory.as_deref() {
        None => home.join("Desktop/local_server"),
        Some(path) if path == std::path::Path::new("Downloads") => home.join("Downloads"),
        Some(path) => path.to_path_buf(),
    };

    let root = std::fs::canonicalize(&directory)?;

    if !root.is_dir() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Not a directory",
        ));
    }

    let app = Router::new()
        .route("/", get(list_files))
        .route("/files/{*filename}", get(file_info))
        .route("/browse/{*directory}", get(list_files))
        .with_state(root);

    let ip = if cli.lan {
        [0, 0, 0, 0]
    } else {
        [127, 0, 0, 1]
    };

    let address = SocketAddr::from((ip, cli.port));
    let listener = tokio::net::TcpListener::bind(address).await?;
    println!("Listening on http://{}", listener.local_addr()?);

    if cli.qr {
        let ip = local_ip().map_err(std::io::Error::other)?;
        let url = format!("http://{}:{}", ip, cli.port);

        let code = QrCode::new(url.as_bytes()).map_err(std::io::Error::other)?;

        let image = code
            .render::<unicode::Dense1x2>()
            .dark_color(unicode::Dense1x2::Light)
            .light_color(unicode::Dense1x2::Dark)
            .build();

        println!("Open on your phone: {url}");
        println!("{image}");
    }

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    println!("Server stopped.");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_validation() {
        let valid = [
            "hello.txt",
            "documents/report.pdf",
            "photos/vacation/image.jpg",
            "folder with spaces/file.txt",
        ];
        let invalid = [
            "",
            ".",
            "..",
            "../secret.txt",
            "documents/../secret.txt",
            ".env",
            "documents/.hidden",
            "documents//report.pdf",
            "/etc/passwd",
            r"documents\report.pdf",
        ];

        for path in valid {
            assert!(is_valid_relative_path(path), "Should accept: {path}");
        }
        for path in invalid {
            assert!(!is_valid_relative_path(path), "Should reject: {path}");
        }
    }

    #[test]
    fn test_io_error_to_status() {
        let cases = [
            (std::io::ErrorKind::NotFound, StatusCode::NOT_FOUND),
            (std::io::ErrorKind::PermissionDenied, StatusCode::FORBIDDEN),
            (
                std::io::ErrorKind::AlreadyExists,
                StatusCode::INTERNAL_SERVER_ERROR,
            ),
        ];

        for (kind, expected) in cases {
            let error = std::io::Error::from(kind);
            assert_eq!(io_error_to_status(error), expected);
        }
    }

    #[tokio::test]
    async fn test_file_handler() -> std::io::Result<()> {
        let temp = tempfile::tempdir()?;
        let shared = temp.path().join("shared");
        std::fs::create_dir(&shared)?;
        std::fs::write(shared.join("hello.txt"), "Hello from Rust")?;
        std::fs::create_dir(shared.join("documents"))?;

        let root = std::fs::canonicalize(&shared)?;
        let response = file_info(State(root.clone()), Path("hello.txt".to_string()))
            .await
            .expect("Regular file should be accessible")
            .into_response();
        assert_eq!(response.status(), StatusCode::OK);

        // Directories should not be downloadable
        let result = file_info(State(root.clone()), Path("documents".to_string())).await;
        assert_eq!(result.err(), Some(StatusCode::NOT_FOUND));
        // Directory traversal should be rejected
        let result = file_info(State(root.clone()), Path("../outside.txt".to_string())).await;
        assert_eq!(result.err(), Some(StatusCode::BAD_REQUEST));
        // Symlinks escaping the shared directory should be rejected
        #[cfg(unix)]
        {
            let outside = temp.path().join("secret.txt");
            std::fs::write(&outside, "Private information")?;
            std::os::unix::fs::symlink(&outside, shared.join("secret-link.txt"))?;
            let result = file_info(State(root), Path("secret-link.txt".to_string())).await;
            assert_eq!(result.err(), Some(StatusCode::FORBIDDEN));
        }

        Ok(())
    }
}
