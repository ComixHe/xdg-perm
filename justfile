lint:
    cargo fmt --all -- --check
    taplo format --check
    cargo clippy -- -D warnings

format:
    cargo fmt
    taplo format

release:
    cargo build --release
