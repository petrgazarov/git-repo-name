.PHONY: test run

test:
	cargo test -- --test-threads=1 $(ARGS)

run:
	cargo run -- $(ARGS)