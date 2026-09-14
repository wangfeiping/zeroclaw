.PHONY: tests install version

export GIT_TAG := $(shell git describe --tags --always --dirty=-dirty --abbrev=7)
#export RUST_VERSION := $(shell rustc --version)

# Stamps Cargo.toml's [workspace.package] version (and path-dep version
# pins) from GIT_TAG so the built binary's `--version` reflects the exact
# commit it came from.
version:
	scripts/release/set-version-from-git-tag.sh

# Builds the Ratchet binary in release (optimized).
#
# Binaries will most likely be found in `./target/release`
install: version
	install.sh --source --preset all

