.PHONY: test run

test:
  # --nocapture is needed for tests that assert stdout
	cargo test -- --test-threads=1 --nocapture $(ARGS)

run:
	cargo run -- $(ARGS)

build_release:
	cargo build --release $(ARGS)