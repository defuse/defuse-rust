# defuse-rust

Rust rewrite of defuse.ca.

## Install dependencies:

- [Rust](https://rustup.rs) (stable)
- vim (for syntax highlighting)
- ruby (for big number calculator)
- gcc (for online assembler)
- objdump (for online assembler)

```bash
sudo apt-get install vim ruby gcc binutils
```

## Development Setup

Clone `defuse-rust` and `defuse-tester`:

```
git clone git@github.com:defuse/defuse-rust
git clone git@github.com:defuse/defuse-tester
```

### Running the Dev Server

For this part, `cd` into `defuse-rust`:

```bash
cd defuse-rust
```

```bash
# 1. Start the database (first run creates all databases and tables)
cd dev
docker compose up -d
cd ..

# 2. Copy the dev environment file to the project root
cp dev/dotenv-example .env

# 3. Source the .env file to set necessary environment variables
# We need to export the environment variables so that they are available
# to child proceses (like when running the tests)
set -a && source .env && set +a

# 4. Run the unit tests (there may be expected failures with --include-ignored)
cargo test --no-fail-fast -- --include-ignored

# 5. Run the server
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

While the server and database are running, in another terminal, `cd` into
`defuse-tester`:

```bash
cd ../defuse-tester
```

Then, to run all of the integration tests, simply run:

```
DEFUSE_URL=http://localhost:3000/ cargo test --no-fail-fast  -- --include-ignored
```

Note: If you want to run the integration tests against the production server,
you need to put the CAPTCHA bypass key at `secrets/captcha-bypass-key.txt`.

You can also take a snapshots to compare against the PHP version:

```
cargo run --bin snapshot http://localhost:3000/ --normalize-vim-tags new-snapshot-dir
```

And the benchmarks/stress tests:

```
cargo run --bin stress http://localhost:3000/
```

### Measuring Code Coverage

Requires [cargo-llvm-cov](https://github.com/taiki-e/cargo-llvm-cov):

```bash
cargo install cargo-llvm-cov
```

From the `defuse-rust` directory, with the database already running:

**Terminal 1** — build and run the instrumented server:

```bash
cargo llvm-cov clean
cargo llvm-cov run --release
```

**Terminal 2** — run the integration tests:

```bash
cd ../defuse-tester
DEFUSE_URL=http://localhost:3000/ cargo test --no-fail-fast -- --include-ignored
```

After the tests finish, Ctrl+C the server in Terminal 1, then generate the
report:

```bash
cargo llvm-cov report --release --html
# Report is at target/llvm-cov-target/html/index.html
```
