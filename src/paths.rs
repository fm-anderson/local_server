use axum::http::StatusCode;
use percent_encoding::{NON_ALPHANUMERIC, utf8_percent_encode};
use std::path::{Path, PathBuf};

pub fn is_valid_relative_path(path: &str) -> bool {
    !path.contains('\\')
        && path
            .split('/')
            .all(|part| !part.is_empty() && !part.starts_with('.'))
}

pub fn is_valid_filename(name: &str) -> bool {
    !name.contains('/') && is_valid_relative_path(name)
}

pub fn encode_path(path: &str) -> String {
    path.split('/')
        .filter(|part| !part.is_empty())
        .map(|part| utf8_percent_encode(part, NON_ALPHANUMERIC).to_string())
        .collect::<Vec<_>>()
        .join("/")
}

pub fn candidate_filename(filename: &str, attempt: usize) -> String {
    if attempt == 0 {
        return filename.to_string();
    }

    let path = Path::new(filename);
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(filename);

    match path.extension().and_then(|e| e.to_str()) {
        Some(ext) => format!("{stem}_{attempt}.{ext}"),
        None => format!("{stem}_{attempt}"),
    }
}

pub async fn create_unique_file(
    directory: &Path,
    filename: &str,
) -> std::io::Result<(tokio::fs::File, PathBuf)> {
    for attempt in 0.. {
        let candidate = candidate_filename(filename, attempt);
        let path = directory.join(candidate);

        match tokio::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .await
        {
            Ok(file) => return Ok((file, path)),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    }

    unreachable!()
}

pub async fn resolve_directory(root: &Path, relative: &str) -> Result<PathBuf, StatusCode> {
    if !relative.is_empty() && !is_valid_relative_path(relative) {
        return Err(StatusCode::BAD_REQUEST);
    }

    let current = tokio::fs::canonicalize(root.join(relative))
        .await
        .map_err(io_error_to_status)?;

    if !current.starts_with(root) {
        return Err(StatusCode::FORBIDDEN);
    }

    if !current.is_dir() {
        return Err(StatusCode::NOT_FOUND);
    }

    Ok(current)
}

pub fn io_error_to_status(error: std::io::Error) -> StatusCode {
    match error.kind() {
        std::io::ErrorKind::NotFound => StatusCode::NOT_FOUND,
        std::io::ErrorKind::PermissionDenied => StatusCode::FORBIDDEN,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    }
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
    fn test_filename_validation() {
        let valid = ["hello.txt", "photo 1.jpg", "archive.tar.gz", "README"];
        let invalid = [
            "",
            ".",
            "..",
            ".hidden",
            "folder/file.txt",
            r"folder\file.txt",
            "../secret.txt",
        ];

        for name in valid {
            assert!(is_valid_filename(name), "Should accept: {name}");
        }
        for name in invalid {
            assert!(!is_valid_filename(name), "Should reject: {name}");
        }
    }

    #[test]
    fn test_candidate_filename() {
        assert_eq!(candidate_filename("photo.jpg", 0), "photo.jpg");
        assert_eq!(candidate_filename("photo.jpg", 1), "photo_1.jpg");
        assert_eq!(candidate_filename("photo.jpg", 2), "photo_2.jpg");
        assert_eq!(candidate_filename("README", 1), "README_1");
        assert_eq!(candidate_filename("archive.tar.gz", 1), "archive.tar_1.gz");
    }

    #[tokio::test]
    async fn test_create_unique_file_auto_rename() -> std::io::Result<()> {
        let temp = tempfile::tempdir()?;
        let dir = temp.path();

        let (_, first) = create_unique_file(dir, "photo.jpg").await?;
        assert_eq!(first.file_name().unwrap(), "photo.jpg");

        let (_, second) = create_unique_file(dir, "photo.jpg").await?;
        assert_eq!(second.file_name().unwrap(), "photo_1.jpg");

        let (_, third) = create_unique_file(dir, "photo.jpg").await?;
        assert_eq!(third.file_name().unwrap(), "photo_2.jpg");

        Ok(())
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
}
