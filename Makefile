.PHONY: all build check fmt clippy test clean install run

all: check build

build:
	cargo build --workspace --release

run:
	cargo run -p duir-tui --release

check: fmt clippy test

fmt:
	cargo fmt --all -- --check

clippy:
	cargo clippy --workspace --all-targets -- -D warnings

test:
	cargo test --workspace

clean:
	cargo clean

install:
	cargo install --path crates/duir-tui
