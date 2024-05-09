[![Rust](https://github.com/softberries/s3tui/actions/workflows/rust.yml/badge.svg)](https://github.com/softberries/s3tui/actions/workflows/rust.yml)

---

# s3tui (WIP) - AWS S3 Transfer CLI

**Please note that this project has no releases yet and its a work in progress. Use at your own risk.**

`s3tui` is a ~~powerful~~ terminal-based application that enables seamless file transfers between your local machine and multiple AWS S3 accounts. Crafted with the [ratatui](https://github.com/ratatui-org/ratatui) Rust TUI framework, `s3tui` provides a robust user interface for managing uploads and downloads simultaneously in both directions, enhancing your productivity with S3 services.

![s3tui](assets/s3tui.gif)

## Features

- **Multiple Account Support**: Easily configure and switch between different S3 accounts during runtime using the 's' command.
- **Simultaneous Transfers**: Transfer multiple files at once, both to and from S3, thanks to multithreading capabilities powered by the [tokio](https://github.com/tokio-rs/tokio) library.
- **Interactive Commands**:
    - `s` - Move back to the file manager window.
    - `Esc` - Select or deselect files for transfer.
    - `↔ / j / k` - Navigate up or down the file lists.
    - `t` - Display currently selected files for transfer.
    - `r` - Execute the selected transfers.
    - `q` - Quit the application.
    - `?` - Access the help page with all available commands.
- **Environment Configuration**: Customize settings via environment variables or utilize default settings compliant with the XDG Base Directory Specification.
- **Error Handling**: Integrated `color_eyre` panic hook for clear and colorized error reporting.
- **Version Information**: Quickly view the application version with the `--version` command.

## Setup

1. **Configure Environment Variables**:
   ```bash
   export S3TUI_CONFIG=`pwd`/.config
   export S3TUI_DATA=`pwd`/.data
   export S3TUI_LOGLEVEL=info
   ```
   Alternatively, use the default paths set according to the XDG Base Directory Specification.

2. **Installation**:
    - Ensure you have Rust and `cargo` installed.
    - Clone the repository and build the project:
      ```bash
      git clone <repository-url>
      cd s3tui
      cargo build --release
      ```

3. **Running s3tui**:
    - Navigate to the project directory and run:
      ```bash
      ./target/release/s3tui
      ```

## Logs

Application logs are efficiently managed and stored in the directory specified by `S3TUI_DATA`, keeping you informed of all operations and aiding in troubleshooting.

## Getting Started

Once `s3tui` is running, press `?` to open the help page, which displays all the commands and their functions, allowing you to start transferring files immediately.

Enhance your productivity with `s3tui`, the command-line interface that bridges the gap between local file management and cloud storage with ease and efficiency. Whether you're managing large datasets or performing routine backups, `s3tui` makes S3 file transfer tasks intuitive and manageable directly from your terminal.
