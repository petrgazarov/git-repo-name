.PHONY: test run

test:
  # --nocapture is needed for tests that assert stdout
	cargo test $(CARGO_OPTS) -- --test-threads=1 --nocapture $(ARGS)

run:
	cargo run $(CARGO_OPTS) -- $(ARGS)

build_release:
	cargo build --release $(CARGO_OPTS)

bump_patch_version:
	@current_version=$$(grep '^version *= *"' Cargo.toml | sed -E 's/version *= *"([0-9]+)\.([0-9]+)\.([0-9]+)".*/\1 \2 \3/') ;\
	set -- $$current_version ;\
	new_version=$$1.$$2.$$(( $$3 + 1 )) ;\
	sed -i '' -E "s/(^version *= *\")[0-9]+\.[0-9]+\.[0-9]+(\".*)/\1$${new_version}\2/" Cargo.toml ;\
	echo "Bumped version to $$new_version"
