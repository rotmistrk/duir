.PHONY: all build check fmt clippy test clean install install-local run

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

install-local: build
	mkdir -p ~/.local/bin
	cp target/release/duir-tui ~/.local/bin/duir
	@echo "Installed to ~/.local/bin/duir"
