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
