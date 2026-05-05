.PHONY: all build check fmt clippy test clean install install-local run lint-length lint-density lint-safety

MAX_CODE_LINES = 240

all: check build

build:
	cargo build --workspace --release

run:
	cargo run -p duir-tui --release

check: fmt clippy lint-length lint-safety test

fmt:
	cargo fmt --all -- --check

clippy:
	cargo clippy --workspace --all-targets -- -D warnings

lint-length:
	@echo "=== code lines check (max $(MAX_CODE_LINES) non-blank non-comment lines) ==="
	@fail=0; \
	for f in $$(find crates -name '*.rs'); do \
		n=$$(grep -cvE '^\s*$$|^\s*//' "$$f" || echo 0); \
		if [ "$$n" -gt $(MAX_CODE_LINES) ]; then \
			echo "FAIL: $$f ($$n code lines > $(MAX_CODE_LINES))"; \
			fail=1; \
		fi; \
	done; \
	[ "$$fail" -eq 0 ] || { echo "Split files above $(MAX_CODE_LINES) code lines"; exit 1; }

lint-safety:
	@echo "=== production safety check (no unwrap/expect/assert/panic) ==="
	@fail=0; \
	for f in $$(find crates -name '*.rs' ! -path '*/tests/*' ! -path '*/tests.rs' ! -name '*_tests.rs' ! -name '*_test.rs'); do \
		prod=$$(sed '/^#\[cfg(test)\]/,$$d' "$$f"); \
		hits=$$(echo "$$prod" | grep -nE '\.unwrap\(\)|\.expect\(|assert!|assert_eq!|assert_ne!|panic!\(' || true); \
		if [ -n "$$hits" ]; then \
			echo "FAIL: $$f"; \
			echo "$$hits" | head -5; \
			fail=1; \
		fi; \
	done; \
	[ "$$fail" -eq 0 ] || { echo "Production code must not contain unwrap/expect/assert/panic"; exit 1; }

test:
	cargo test --workspace

clean:
	cargo clean

install: build
	sudo install -m 755 target/release/duir-tui /usr/local/bin/duir
	@echo "Installed to /usr/local/bin/duir"

install-local: build
	mkdir -p ~/.local/bin
	install -m 755 target/release/duir-tui ~/.local/bin/duir
	@echo "Installed to ~/.local/bin/duir"
