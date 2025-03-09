.PHONY: test run

test:
  # --nocapture is needed for tests that assert stdout
	cargo test $(CARGO_OPTS) -- --test-threads=1 --nocapture $(ARGS)

run:
	cargo run $(CARGO_OPTS) -- $(ARGS)

build_release:
	cargo build --release $(CARGO_OPTS)