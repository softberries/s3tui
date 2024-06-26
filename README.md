<div style="text-align: center;">

            _____ __        _ 
       ____|__  // /___  __(_)
      / ___//_  / __/ / / / / 
     (__  )__/ / /_/ /_/ / /  
    /____/____/\__/\__,_/_/

</div>
<div style="text-align: center;">

[![CI][s0]][l0] [![crates][s1]][l1] ![MIT][s2] [![UNSAFE][s3]][l3] [![TWEET][s6]][l6] [![dep_status][s7]][l7]

</div>

[s0]: https://github.com/softberries/s3tui/actions/workflows/rust.yml/badge.svg

[l0]: https://github.com/softberries/s3tui/actions/workflows/rust.yml

[s1]: https://img.shields.io/crates/v/s3tui.svg

[l1]: https://crates.io/crates/s3tui

[s2]: https://img.shields.io/badge/license-MIT-blue.svg

[s3]: https://img.shields.io/badge/unsafe-forbidden-success.svg

[l3]: https://github.com/rust-secure-code/safety-dance/

[s6]: https://img.shields.io/twitter/follow/grajo?label=follow&style=social

[l6]: https://twitter.com/intent/follow?screen_name=grajo

[s7]: https://deps.rs/repo/github/softberries/s3tui/status.svg

[l7]: https://deps.rs/crate/s3tui

---

# s3tui - AWS S3 Transfer CLI

`s3tui` is a ~~powerful~~ terminal-based application that enables seamless file transfers between your local machine and
multiple AWS S3 accounts. Crafted with the [ratatui](https://github.com/ratatui-org/ratatui) Rust TUI framework, `s3tui`
provides a robust user interface for managing uploads and downloads simultaneously in both directions, enhancing your
productivity with S3 services.

![s3tui](assets/s3tui.gif)

## Features

- **Multiple Account Support**: Easily configure and switch between different S3 accounts during runtime using the 's'
  command.
- **Simultaneous Transfers**: Transfer multiple files at once, both to and from S3, thanks to multithreading
  capabilities powered by the [tokio](https://github.com/tokio-rs/tokio) library.
- **Interactive Commands**:
    - `Tab,↔` - move between local and s3 panel
    - `s` - select account currently in use.
    - `Esc` - move back to the file manager window.
    - `↕ / j / k` - move up/down on the lists.
    - `t` - select/deselect files to transfer.
    - `c` - create bucket.
    - `⌫ / Del` - delete item.
    - `l` - Display currently selected files for transfer.
    - `r` - Execute the selected transfers.
    - `q` - Quit the application.
    - `?` - Access the help page with all available commands.
- **Environment Configuration**: Customize settings via environment variables or utilize default settings compliant with
  the XDG Base Directory Specification.
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

   You can check your configuration by running `s3tui --version` which will show you the paths currently in use.

2. **Add your s3 credentials**
    - Add as many configurations under `creds` directory (inside your `.data` directory specified with `S3TUI_DATA` env variable)
    - The file should look like the one below:
   
```bash
access_key=YOUR_ACCESS_KEY
secret_key=YOUR_SECRET_KEY
default_region=eu-west-1
```
Make sure there is a new line at the end and there are no leading spaces on the lines.

3. **Installation from crates.io**:
    - Ensure you have Rust and `cargo` installed.
    - Install with cargo
    ```bash
      cargo install s3tui
    ```
4. **Building locally**:
    - Ensure you have Rust and `cargo` installed.
    - Clone the repository and build the project:
    ```bash
      git clone <repository-url>
      cd s3tui
      cargo build --release
    ```

5. **Running s3tui**:
- Navigate to the project directory and run:
```bash
./target/release/s3tui
```

## Logs

Application logs are efficiently managed and stored in the directory specified by `S3TUI_DATA`, keeping you informed of
all operations and aiding in troubleshooting.

## Getting Started

Once `s3tui` is running, press `?` to open the help page, which displays all the commands and their functions, allowing
you to start transferring files immediately.

Enhance your productivity with `s3tui`, the command-line interface that bridges the gap between local file management
and cloud storage with ease and efficiency. Whether you're managing large datasets or performing routine
backups, `s3tui` makes S3 file transfer tasks intuitive and manageable directly from your terminal.
