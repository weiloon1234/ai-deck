# Compatible with the GNU Make 3.81 bundled with Apple's command-line tools.
# package.json remains the source of truth for existing build/check/test commands.
SHELL := /bin/sh
.DEFAULT_GOAL := help
.NOTPARALLEL:
# npm does not forward Make jobserver file descriptors to Cargo.
unexport MAKEFLAGS MFLAGS

.PHONY: help doctor setup dev preview contracts check test test-ui test-runtime verify build clean dependencies

help: ## Show available commands (also the default for make)
	@printf 'AI Deck — macOS development\n\n'
	@awk 'BEGIN { FS = ":.*## " } /^[a-zA-Z][a-zA-Z0-9_-]*:.*## / { printf "  make %-15s %s\n", $$1, $$2 }' Makefile
	@printf '\nFirst checkout: make setup\nBefore sharing changes: make verify\n'

doctor: ## Check local Mac tools without installing or changing anything
	@test "$$(uname -s)" = Darwin || { echo 'Desktop development currently requires macOS.' >&2; exit 1; }
	@for tool in node npm rustc cargo python3 xcode-select xcrun; do \
		command -v "$$tool" >/dev/null 2>&1 || { echo "Missing $$tool. See README.md: Develop and build." >&2; exit 1; }; \
	done
	@xcode-select -p >/dev/null 2>&1 && xcrun --find clang >/dev/null 2>&1 && xcrun --sdk macosx --show-sdk-path >/dev/null 2>&1 || { echo 'Configure Xcode or its command-line tools and the macOS SDK. See README.md.' >&2; exit 1; }
	@node -e 'const lock = require("./package-lock.json"); console.log("Node " + process.version + " (Vite requires " + lock.packages["node_modules/vite"].engines.node + ")");'
	@npm --version
	@rustc --version
	@cargo --version
	@cargo fmt --version || { echo 'Rustfmt is required. For rustup installations: rustup component add rustfmt' >&2; exit 1; }
	@cargo clippy --version || { echo 'Clippy is required. For rustup installations: rustup component add clippy' >&2; exit 1; }
	@python3 -c 'import sys; sys.exit("Python 3.9 or newer is required for the runtime tests.") if sys.version_info < (3, 9) else print("Python " + sys.version.split()[0])'
	@printf 'Local tools found. make setup enforces dependency engine requirements.\n'

setup: doctor ## Install locked npm dependencies and fetch locked Rust crates
	npm ci --engine-strict
	cargo fetch --locked --manifest-path src-tauri/Cargo.toml

# Do not run npm installation implicitly when starting a check or the application.
dependencies:
	@test -f node_modules/.package-lock.json || { echo 'Project dependencies are missing. Run make setup first.' >&2; exit 1; }

dev: dependencies ## Run the desktop app with development reload
	npm run tauri -- dev

preview: dependencies ## Run the browser preview; native actions are unavailable
	npm run dev

contracts: dependencies ## Regenerate shared types, defaults and catalog schema
	npm run contracts

check: dependencies ## Check generated files, frontend build, Rust format and lints
	npm run check

test: dependencies ## Run all regular local Rust, Python and Vue tests
	npm test

test-ui: dependencies ## Run only Vue component tests
	npm run test:ui

test-runtime: ## Run only offline Python supervisor tests
	npm run test:runtime

verify: check test ## Run checks followed by the complete local test suite

build: dependencies ## Build AI Deck.app for this Mac's architecture
	npm run tauri -- build --bundles app

clean: ## Remove project build output; retain dependencies and saved app data
	rm -rf ./dist ./src-tauri/target ./src-tauri/gen
