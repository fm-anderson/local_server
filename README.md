# Local Server

A lightweight HTTP file-sharing server written in **Rust**, built with Axum and Tokio.

Browse directories and download files through a web browser. Share files with other devices on your local network using an optional QR code.

## Features

- **Directory browsing:** Navigate folders through a responsive HTML interface.
- **File downloads:** Download files of any type, with streaming support for large files.
- **Configurable directory:** Share any directory, with shortcuts for common locations.
- **Configurable port:** Change the server's listening port using `-p` or `--port`.
- **LAN access:** Optionally share files with other devices on your network.
- **QR codes:** Generate a terminal QR code for quick access from your phone.
- **Path validation:** Restricts file access to the selected directory, with basic protections against directory traversal.
- **Graceful shutdown:** Stop the server using `Ctrl+C`.

## Requirements

- Rust and Cargo
- macOS or Linux

## Installation

Clone the repository:

```bash
git clone https://github.com/fm-anderson/local_server.git
cd local_server
```

Build the optimized executable:

```bash
cargo build --release
```

The executable will be available at:

```text
target/release/local_server_rs
```

You can also run the application during development using `cargo run`.

## Usage

### Start the server

```bash
./target/release/local_server_rs
```

By default, the server:

- Shares `~/Desktop/local_server`.
- Listens on `127.0.0.1:8000`.
- Accepts connections only from your computer.

The shared directory must already exist.

Open `http://127.0.0.1:8000` in your browser.

### Share a different directory

Use the `Downloads` shortcut:

```bash
./target/release/local_server_rs Downloads
```

This shares `~/Downloads`.

You can also provide an absolute or relative path:

```bash
./target/release/local_server_rs ~/Documents
./target/release/local_server_rs /tmp/shared
./target/release/local_server_rs ./photos
```

Other relative paths are resolved from the current working directory.

### Change the port

The default port is `8000`.

```bash
./target/release/local_server_rs --port 3000
```

Or use the short option:

```bash
./target/release/local_server_rs -p 3000
```

### Share files over your local network

```bash
./target/release/local_server_rs Downloads --lan
```

The `--lan` flag binds the server to all available IPv4 network interfaces.

Find your Mac's local IP address:

```bash
ipconfig getifaddr en0
```

On another device connected to the same network, open:

```text
http://YOUR_LOCAL_IP:8000
```

### Display a QR code

To share files with your phone without manually entering the URL:

```bash
./target/release/local_server_rs Downloads --lan --qr
```

The server prints a local URL and a QR code in the terminal.

Scan the QR code with your phone's camera.

**Note:** `--qr` requires `--lan`.

### Show available options

```bash
./target/release/local_server_rs --help
```

Check the application version:

```bash
./target/release/local_server_rs --version
```

### Stop the server

Press `Ctrl+C`.

The server initiates a graceful shutdown, allowing active requests to finish.

## Security

Local Server is intended for personal use and trusted environments.

- LAN access is disabled by default.
- There is **no authentication or HTTPS**.
- Hidden files and directories are not exposed through the browser.
- Symbolic links are displayed but are not clickable.
- File paths are validated to prevent basic directory traversal and access outside the shared root.

Filesystem path validation does not eliminate every race condition involving concurrent file or symlink changes.

**Only enable `--lan` on trusted networks and share directories that do not contain sensitive files.** Binding to all interfaces can expose the server beyond your immediate LAN, depending on your network configuration.

## Project Structure

```text
local_server/
├── src/
│   └── main.rs         # CLI, HTTP handlers, and tests
├── templates/
│   └── index.html      # Askama HTML template
├── Cargo.toml          # Rust dependencies and configuration
├── Cargo.lock          # Locked dependency versions
└── README.md
```

The HTML interface uses Askama templates, which are compiled into the executable.

## Technology

- **Rust** — Application language
- **Axum** — HTTP routing and responses
- **Tokio** — Asynchronous runtime
- **Clap** — Command-line argument parsing
- **Askama** — HTML templates
- **qrcode** — Terminal QR code generation

## License

This project is licensed under the [MIT License](LICENSE.md).
