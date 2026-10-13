# Native test guidance

Inherits [native guidance](../AGENTS.md). Regular tests must remain independent of installed coding CLIs, working account state and cloud access.

- Reuse `support::Fixture`, fake provider/runtime/credential implementations and disposable `tempfile` directories. Fake credentials must be clearly disposable; never call `OsCredentialStore` or read the owner's app-data directory.
- Lifecycle tests should assert persisted intent, ownership, provider absence and process cleanup at observable boundaries. Cover uncertain API responses without weakening production validation or retrying paid creation.
- Use bounded loopback fixtures for HTTP behavior; exercise bad authentication, model mismatch and sanitized errors. Do not replace a denied local listener with an external endpoint.
- PTY/process regressions must stop and reap every fixture process, including detached descendants. Verify unrelated sessions survive targeted cleanup; keep explicit deadlines so a failing test does not leave children behind.
- Catalog evidence in fixtures is labeled fixture-only. Assert fingerprint/CLI/GPU changes invalidate it and capacity-dependent checks stay enforced. Never write fixture evidence into the bundled catalog.
- `installed_cli_smoke.rs` and the public metadata probe are ignored intentionally. Inspect and select the specific probe before running it; document its exact versions and whether it tested rejection/routing, metadata retrieval or actual inference.

Use `cargo test --manifest-path src-tauri/Cargo.toml --test <suite>` for focused checks, then the root verification workflow for cross-layer changes. Keep test results and environment limitations in [local validation](../../docs/local-validation.md).
