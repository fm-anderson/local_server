use clap::Parser;
use std::path::{Path, PathBuf};

#[derive(Parser, Debug)]
#[command(version, about = "Share local files through an HTTP server")]
pub struct Cli {
    /// Directory to share (default: ~/Desktop/local_server)
    pub directory: Option<PathBuf>,

    #[arg(short, long, default_value_t = 8000)]
    pub port: u16,

    #[arg(long)]
    pub lan: bool,

    #[arg(long)]
    pub qr: bool,
}

impl Cli {
    pub fn validate_and_resolve_root(&self) -> std::io::Result<PathBuf> {
        if self.qr && !self.lan {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "--qr requires --lan",
            ));
        }

        let home = std::env::var_os("HOME")
            .map(PathBuf::from)
            .ok_or_else(|| std::io::Error::other("HOME is not set"))?;

        let directory = match self.directory.as_deref() {
            None => home.join("Desktop/local_server"),
            Some(path) if path == Path::new("Downloads") => home.join("Downloads"),
            Some(path) => path.to_path_buf(),
        };

        let root = std::fs::canonicalize(&directory)?;

        if !root.is_dir() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Not a directory",
            ));
        }

        Ok(root)
    }
}
