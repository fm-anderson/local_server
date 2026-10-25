use askama::Template;
use axum::{
    Router,
    extract::{Path, State},
    http::{StatusCode, header},
    response::{Html, IntoResponse},
    routing::get,
};
use clap::Parser;
use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};
use std::path::PathBuf;

#[derive(Parser, Debug)]
struct Cli {
    #[arg(default_value = ".")]
    directory: PathBuf,
}

struct FileEntry {
    name: String,
    href: String,
    kind: &'static str,
    is_clickable: bool,
}

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate {
    entries: Vec<FileEntry>,
}

async fn file_info(
    State(root): State<PathBuf>,
    Path(filename): Path<String>,
) -> Result<impl IntoResponse, StatusCode> {
    if filename.contains('\\')
        || filename
            .split('/')
            .any(|part| part.is_empty() || part.starts_with('.'))
    {
        return Err(StatusCode::BAD_REQUEST);
    }

    let file_path: PathBuf = tokio::fs::canonicalize(root.join(&filename))
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    if !file_path.starts_with(&root) {
        return Err(StatusCode::FORBIDDEN);
    }

    let contents = tokio::fs::read(&file_path)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    let content_type = mime_guess::from_path(&file_path)
        .first_or_octet_stream()
        .to_string();

    Ok((
        [
            (header::CONTENT_TYPE, content_type),
            (header::CONTENT_DISPOSITION, "attachment".to_string()),
        ],
        contents,
    ))
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let cli = Cli::parse();

    let root = std::fs::canonicalize(&cli.directory)?;

    if !root.is_dir() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "Not a directory",
        ));
    }

    async fn list_files(
        State(root): State<PathBuf>,
        directory: Option<Path<String>>,
    ) -> Result<Html<String>, StatusCode> {
        let relative = directory.map(|Path(value)| value).unwrap_or_default();

        if !relative.is_empty()
            && (relative.contains('\\')
                || relative
                    .split('/')
                    .any(|part| part.is_empty() || part.starts_with('.')))
        {
            return Err(StatusCode::BAD_REQUEST);
        }

        let current = tokio::fs::canonicalize(root.join(&relative))
            .await
            .map_err(|_| StatusCode::NOT_FOUND)?;

        if !current.starts_with(&root) {
            return Err(StatusCode::FORBIDDEN);
        }

        if !current.is_dir() {
            return Err(StatusCode::NOT_FOUND);
        }

        let mut directory = tokio::fs::read_dir(&current)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        let mut entries = Vec::new();

        while let Some(entry) = directory
            .next_entry()
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        {
            let name = entry.file_name().to_string_lossy().into_owned();

            if name.starts_with('.') {
                continue;
            }

            let file_type = entry
                .file_type()
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

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
            });
        }

        entries.sort_by(|a, b| a.name.cmp(&b.name));

        let html = IndexTemplate { entries }
            .render()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        Ok(Html(html))
    }

    let app = Router::new()
        .route("/", get(list_files))
        .route("/files/{*filename}", get(file_info))
        .route("/browse/{*directory}", get(list_files))
        .with_state(root);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:8000").await?;

    println!("Listening on http://127.0.0.1:8000");

    axum::serve(listener, app).await
}
