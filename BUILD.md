# Build

This project uses the stable Rust toolchain (1.86 as of 5/12/25).

## Bare Metal

If you're not using devcontainers, you'll need to install Rust and the necessary dependencies on your system.

### Prerequisites

- Rust toolchain (1.86 or newer)
- PostgreSQL
- Redis or Valkey
- SQLx CLI: `cargo install sqlx-cli@0.8.3 --no-default-features --features postgres`

### Common Commands

- Build: `cargo build`
- Check: `cargo check`
- Lint: `cargo clippy`
- Run tests: `cargo test`
- Run server: `cargo run --bin smokesignal`
- Run with debug: `RUST_BACKTRACE=1 RUST_LOG=debug cargo run`
- Run database migrations: `sqlx migrate run`

### Build Options

- Build with embedded templates: `cargo build --bin smokesignal --no-default-features -F embed`
- Build with template reloading: `cargo build --bin smokesignal --no-default-features -F reload`

## Devcontainers (Recommended)

The easiest way to get started is by using the provided devcontainer configuration with Visual Studio Code.

### Setup

1. Install [Visual Studio Code](https://code.visualstudio.com/)
2. Install the [Dev Containers extension](https://marketplace.visualstudio.com/items?itemName=ms-vscode-remote.remote-containers)
3. Clone this repository
4. Open the repository in VS Code
5. When prompted, click "Reopen in Container" or run the "Dev Containers: Reopen in Container" command

The devcontainer will set up the following services:
- Rust development environment with all dependencies
- PostgreSQL database
- Valkey (Redis-compatible) key-value store 
- Tailscale networking (optional)

### Disabling Tailscale

If you don't need the Tailscale service, you can disable it by:

1. Open `.devcontainer/docker-compose.yml`
2. Comment out or remove the `tailscale` service section
3. Rebuild the devcontainer (Command Palette > "Dev Containers: Rebuild Container")

### VS Code Configuration

The devcontainer comes with recommended VS Code extensions for Rust development:
- Rust Analyzer
- Jinja HTML
- Even Better TOML

A launch configuration example is provided in `.vscode/launch.example.json`. Copy this to `.vscode/launch.json` to enable debugging in VS Code.

## Development Configuration

The application requires several environment variables for cryptographic operations. You can generate appropriate values using the included `crypto` binary.

### Generating Cryptographic Keys

Generate a random 64-byte key encoded in base64:

```
cargo run --bin crypto -- key
```

Generate a JWK (JSON Web Key):

```
cargo run --bin crypto -- jwk
```

The generated JWK should be added to a JWKS (JSON Web Key Set) in the file `keys.json`:

```json
{
    "keys": [
      { "kid": "01J7PM272ZF0DYZAPR3499VBTM" ...},
      { "kid": "01J8G3J3CDVJ15C63PMCDS3K97" ...},
      { "kid": "01JF2QS2S86SG2R23HTZ0JKB76" ...}
    ]
  }
  ```

### Environment Variables

Set the following environment variables with values generated from the commands above:

- `SIGNING_KEYS`: The path to the `keys.json` file
- `OAUTH_ACTIVE_KEYS`: A comma seperated list of JWK IDs used to actively sign OAuth sessions
- `DESTINATION_KEY`: A JWK ID used to sign destination (used in redirects) values
- `HTTP_COOKIE_KEY`: A key used to encrypt HTTP sessions

You can add these to your .env file or set them directly in your environment.

### Additional Configuration for Airgapped Development

For airgapped development, you can configure:

- `PLC_HOSTNAME`: Custom PLC hostname for development
- `DNS_NAMESERVERS`: Custom DNS nameservers
- `ADMIN_DIDS`: Comma-separated list of admin DIDs

Example:
```
PLC_HOSTNAME=localhost:3000
DNS_NAMESERVERS=1.1.1.1,1.0.0.1
ADMIN_DIDS=did:plc:yourdevdid1,did:plc:yourdevdid2
```