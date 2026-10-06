# Native Rust guidance

Inherits [root guidance](../AGENTS.md). Rust owns durable state, cloud operations, validation and local processes.

- Keep `commands.rs` a thin Tauri boundary. Put domain behavior in the existing focused `AppCore` services. Register new commands in `lib.rs` and export shared types through `examples/export_contracts.rs` when needed.
- Reuse `RunpodApi`, `RuntimeApi`, `CredentialStore`, `HuggingFaceApi` and `ModelAnalyzer` boundaries. Extend existing clients and fixtures rather than creating parallel cloud or credential implementations.
- Write state through `StateStore::update`. Preserve atomic writes, private permissions, the exclusive app lock and failure behavior that leaves existing state intact. Add default-compatible fields or an explicit non-destructive migration; test old saved records.
- Keep network/process work outside the state mutex. Respect the deployment operations lock and separate model-import lock. Analysis must remain cancellable without blocking Finish; timeouts and quit must clean up owned processes.
- Preserve durable creation intent and reconciliation of uncertain provider responses. Do not retry paid creation blindly. Only modify/delete owned resources and never treat a missing response or stopped process as verified Pod deletion.
- Runtime checks must verify authentication rejection, model identity and inference before Ready/Enable. Compatibility evidence remains bound to the full profile/runtime/launcher fingerprint, GPU and exact CLI version.
- Credentials stay behind `CredentialStore`. Diagnostics and surfaced errors use approved sanitized fields, not raw upstream errors, environment dumps or child-process logs.
- CLI adapters own isolated coding-session configuration. The normal-subscription analyzer is a separate restricted path; keep its authorization, bounded schema, timeout/cancellation and untrusted-metadata handling intact.
- Use explicit command arguments and controlled environments, not shell interpolation. Track and stop only owned process trees. Session resume must wait for prior process cleanup.
- Keep Hub metadata reads bounded and tied to validated Hugging Face URLs and immutable revisions. AI output cannot choose executable code, arbitrary URLs, container images or credential routes. Prices are calculated from provider quotes, never supplied by the analyzer.

Reuse `tests/support` with disposable state and fake providers/credentials. Add focused regressions for lifecycle, storage, metadata and process changes. Run Cargo formatting/lints and applicable tests; for exported types also regenerate contracts and build the frontend. Never weaken production invariants to satisfy a fixture.
