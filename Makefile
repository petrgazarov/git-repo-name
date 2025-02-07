.PHONY: test run

test:
	cargo test -- --test-threads=1

run:
	cargo run -- $(ARGS) 