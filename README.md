# Choard

Choard (chore board) is a local-first household task tracker built in Rust. It
is intended to grow into a configurable Kanban board and workflow engine that
can run on a Raspberry Pi.

## Current status

The initial server foundation is in place: it loads configuration, opens a
SQLite database, and serves `GET /health`. The task board and workflow features
are not implemented yet.

## Getting started

You need the Rust stable toolchain installed. From the project directory, make
your own configuration file by copying the sample:

```sh
cp config.example.toml config.toml
```

Edit `config.toml` to suit your setup, then start the server:

```sh
cargo run
```

By default, the health endpoint is available at
<http://127.0.0.1:8080/health>. It returns `{"status":"ok"}` when the server
can reach SQLite. The database directory is created automatically if needed.

## Configuration

`config.example.toml` is a starter template. Copy it to `config.toml` and edit
your copy; the local `config.toml` is ignored by Git so personal settings are
not accidentally included in commits.

```toml
[server]
host = "127.0.0.1"
port = 8080

[database]
path = "./data/household.db"
```

`server.host` and `server.port` control the listening address. The sample uses
`127.0.0.1`, which only allows connections from the same machine. The server
does not have authentication yet, so do not change this to a network-facing
address such as `0.0.0.0` unless you understand and accept that other devices
on the network may be able to access it.

`database.path` is the SQLite file location. Relative paths are resolved from
the directory where you run Choard.

To load a config file from another location, set `HOUSEHOLD_CONFIG`:

```sh
HOUSEHOLD_CONFIG=/path/to/config.toml cargo run
```

## Development

```sh
cargo fmt --all
cargo check
cargo test
```

GitHub Actions runs formatting checks, checks all Rust targets, Clippy with
warnings denied, tests, and a release build on pushes and pull requests.

The workflow only validates and builds the application; it does not deploy to
the Raspberry Pi. OTA updates can be added later as a separate, deliberate
release pipeline.
