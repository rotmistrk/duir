.PHONY: all build check fmt clippy test clean install install-local run lint-length lint-density lint-safety

MAX_FILE_LINES = 360
MIN_BLANK_PCT = 20

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
	@echo "=== file length check (max $(MAX_FILE_LINES) lines) ==="
	@fail=0; \
	for f in $$(find crates -name '*.rs'); do \
		n=$$(wc -l < "$$f"); \
		if [ "$$n" -gt $(MAX_FILE_LINES) ]; then \
			echo "FAIL: $$f ($$n lines > $(MAX_FILE_LINES))"; \
			fail=1; \
		fi; \
	done; \
	[ "$$fail" -eq 0 ] || { echo "Split files above $(MAX_FILE_LINES) lines"; exit 1; }

lint-density:
	@echo "=== blank line density check (min $(MIN_BLANK_PCT)%) ==="
	@fail=0; \
	for f in $$(find crates -name '*.rs'); do \
		total=$$(wc -l < "$$f"); \
		[ "$$total" -lt 20 ] && continue; \
		blank=$$(grep -c '^$$' "$$f" || true); \
		pct=$$((blank * 100 / total)); \
		if [ "$$pct" -lt $(MIN_BLANK_PCT) ]; then \
			echo "FAIL: $$f ($$pct% blank < $(MIN_BLANK_PCT)%, $$blank/$$total)"; \
			fail=1; \
		fi; \
	done; \
	[ "$$fail" -eq 0 ] || { echo "Add blank lines between logical sections for readability"; exit 1; }

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
