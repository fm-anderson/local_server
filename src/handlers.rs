use crate::paths::{
    create_unique_file, encode_path, io_error_to_status, is_valid_filename, is_valid_relative_path,
    resolve_directory,
};
use askama::Template;
use axum::{
    body::Body,
    extract::{Multipart, Path, State},
    http::{StatusCode, header},
    response::{Html, IntoResponse, Redirect},
};
use std::path::PathBuf;
use tokio::io::AsyncWriteExt;
use tokio_util::io::ReaderStream;

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
    upload_action: String,
}

pub async fn file_info(
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

pub async fn list_files(
    State(root): State<PathBuf>,
    directory: Option<Path<String>>,
) -> Result<Html<String>, StatusCode> {
    let relative = directory.map(|Path(value)| value).unwrap_or_default();
    let current = resolve_directory(&root, &relative).await?;

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

        let entry_path = if relative.is_empty() {
            name.clone()
        } else {
            format!("{relative}/{name}")
        };
        let encoded_path = encode_path(&entry_path);

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
            Some(path) => format!("/browse/{}", encode_path(path)),
            None => "/".to_string(),
        })
    };

    let upload_action = if relative.is_empty() {
        "/upload".to_string()
    } else {
        format!("/upload/{}", encode_path(&relative))
    };

    let html = IndexTemplate {
        entries,
        parent_href,
        upload_action,
    }
    .render()
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Html(html))
}

pub async fn upload_files(
    State(root): State<PathBuf>,
    directory: Option<Path<String>>,
    mut multipart: Multipart,
) -> Result<Redirect, StatusCode> {
    let relative = directory.map(|Path(value)| value).unwrap_or_default();
    let current = resolve_directory(&root, &relative).await?;

    while let Some(mut field) = multipart
        .next_field()
        .await
        .map_err(|_| StatusCode::BAD_REQUEST)?
    {
        let Some(filename) = field.file_name().map(str::to_string) else {
            continue;
        };

        if filename.is_empty() {
            continue;
        }

        if !is_valid_filename(&filename) {
            return Err(StatusCode::BAD_REQUEST);
        }

        let (mut file, file_path) = create_unique_file(&current, &filename)
            .await
            .map_err(io_error_to_status)?;

        let write_result: Result<(), StatusCode> = async {
            while let Some(chunk) = field.chunk().await.map_err(|_| StatusCode::BAD_REQUEST)? {
                file.write_all(&chunk).await.map_err(io_error_to_status)?;
            }
            file.flush().await.map_err(io_error_to_status)?;
            Ok(())
        }
        .await;

        if let Err(status) = write_result {
            drop(file);
            let _ = tokio::fs::remove_file(&file_path).await;
            return Err(status);
        }
    }

    let redirect_target = if relative.is_empty() {
        "/".to_string()
    } else {
        format!("/browse/{}", encode_path(&relative))
    };

    Ok(Redirect::to(&redirect_target))
}

#[cfg(test)]
mod tests {
    use super::*;

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
