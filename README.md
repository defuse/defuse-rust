# defuse-rust

Rust rewrite of defuse.ca

## Requirements

- Rust (stable)
- vim (for syntax highlighting)

## Development Setup

```bash
# Install vim if not already installed
# Ubuntu/Debian:
sudo apt install vim

# macOS:
brew install vim

# Create cache directory for vim highlighting
sudo mkdir -p /storage/vimhl
sudo chown $USER:$USER /storage/vimhl
```

## Running

```bash
# Set required environment variables
export PHPCOUNT_DATABASE_URL="mysql://user:pass@localhost/phpcount"
export UPVOTES_DATABASE_URL="mysql://user:pass@localhost/upvotes"

# Run the server
cargo run
```

## Testing

```bash
cargo test
```

## Production Deployment

This application does not handle TLS directly. It must run behind a TLS-terminating reverse proxy (e.g., Caddy, nginx) in production.

The app expects the proxy to set `X-Forwarded-Proto: https` on HTTPS requests. Without this header, the app assumes HTTP and will redirect to HTTPS, which fails if there's no proxy handling TLS.

**Recommended: Caddy**

```
defuse.ca {
    reverse_proxy localhost:3000
}
```

Caddy automatically provisions Let's Encrypt certificates and sets the correct headers.
