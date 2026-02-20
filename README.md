# defuse-rust

Rust rewrite of defuse.ca.

## Requirements

- Rust (stable)
- vim (for syntax highlighting)
- ruby (for big number calculator)
- gcc (for online assembler)

## Development Setup

Clone `defuse-rust` and `defuse-tester`:

```
git clone git@github.com:defuse/defuse-rust
git clone git@github.com:defuse/defuse-tester
```

### Running the Dev Server

For this part, `cd` into `defuse-tester`:

```bash
cd defuse-tester
```

```bash
# 1. Start the database (first run creates all databases and tables)
cd dev
docker compose up -d
cd ..

# 2. Copy the dev environment file to the project root
cp dev/dotenv-example .env

# 3. Run the server
source .env
cargo run --release
```
By default, `dev/dotenv-example` will use the `dev/test-storage` directory as
the storage directory. This directory contains minimal files in order to make
the defuse-tester integration tests pass.

The dev database container uses `dev/init.sql` to automatically create all
databases, tables, and users on first startup. To reset the database to a clean
state:

```bash
cd dev
docker compose down -v   # -v removes the data volume
docker compose up -d     # re-creates everything from init.sql
```

### Running the Integration Tests

In another terminal, `cd` into `defuse-tester`:

```bash
cd ../defuse-tester
```

Then, to run all of the integration tests, simply run:

```
DEFUSE_URL=http://localhost:3000/ cargo test --no-fail-fast  -- --include-ignored
```

### Measuring Code Coverage

TODO
