# `local_server`

A lightweight, zero-configuration HTTP server written in Ruby to share files from your computer to any device on your local network. It provides a clean, mobile-friendly web interface and a QR code for instant access.

## Features

- **Cross-Platform**: Works on any system with Ruby installed (macOS, Linux, Windows).
- **QR Code Support**: Generate a QR code for easy access to the server URL on mobile devices.
- **Configurable:** Use command-line flags to change the port, show the QR code, or get help.
- **Flexible:** Serve files from any directory on your computer.

## Requirements

- Ruby (tested with Ruby 2.x and 3.x)
- `webrick` (usually included with Ruby)
- `rqrcode-core` gem: This gem is required for the QR code functionality. If you use the `-qr` option and encounter an error, you may need to install it:
  ```bash
  gem install rqrcode
  ```

## Installation

Since `local_server` is a Ruby script, you'll need a Ruby environment set up. Most macOS and Linux distributions come with Ruby pre-installed.

1.  **Save the script**: Save the provided Ruby code into a file named `local_server.rb` (or any other name you prefer).

2.  **Make it executable**:

    ```bash
    chmod +x local_server.rb
    ```

3.  **Move to your PATH**: For easy execution from any directory, move the script to a directory included in your system's `PATH` (e.g., `/usr/local/bin` or `~/bin`).

    ```bash
    mv local_server.rb /usr/local/bin/
    ```

    If `/usr/local/bin` requires superuser permissions, you might need to use `sudo`:

    ```bash
    sudo mv local_server.rb /usr/local/bin/
    ```

## Usage

Once installed, you can run `local_server.rb` directly from your terminal.

### Basic Usage

To serve files from your current directory:

```bash
local_server.rb
```

This will start the server on port `8000` (by default) and display the local URL.

### Serving a Specific Directory

To serve files from a different directory (e.g., your `Downloads` folder):

```bash
local_server.rb ~/Downloads
```

### Specifying a Port

To run the server on a different port (e.g., `3000`):

```bash
local_server.rb -p 3000
```

### Displaying a QR Code

To get a QR code for the server URL, which is handy for quickly accessing it on your phone:

```bash
local_server.rb -qr
```

You can combine options:

```bash
local_server.rb ~/Pictures -p 8080 -qr
```

### Stopping the Server

To stop the server, press `Ctrl+C` in your terminal.

## Examples

- **Sharing a photo album with your phone**:

  ```bash
  local_server ~/Photos/Vacation2024 -qr
  ```

  Scan the QR code with your phone's camera, and you'll be able to browse and download your photos.

- **Quickly transfer a document to another computer on your network**:

  ```bash
  local_server ~/Documents
  ```

  Then, on the other computer, open a web browser and navigate to the IP address and port displayed in your terminal (e.g., `http://192.168.1.100:8000`).

## Contributing

Feel free to open issues [here](https://github.com/fm-anderson/local_server/issues) or submit pull requests on this [repository](https://github.com/fm-anderson/local_server).

## License

This project is licensed under the [MIT License](LICENSE.md).
