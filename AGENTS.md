# Development guidance

## Working agreement

- Read the relevant existing implementation and nearby tests before changing code. Follow its naming, structure and conventions. Read scoped `AGENTS.md` files for directories you edit.
- Keep changes focused. Reuse existing logic, configuration and constants; change their source rather than introducing parallel definitions. Use descriptive filenames and focused functions.
- Ask when requirements are ambiguous. Explain and ask before a large structural change, a new architectural pattern or an irreversible action. Complete already-authorized routine work without repeated permission requests.
- Never run `git commit`, `git push` or `git merge`. The owner manages version control. Read-only Git inspection is allowed. Do not overwrite unrelated work.
- Do not install new packages/dependencies or upgrade toolchains without approval. Use the existing stack: Vue/TypeScript, Tauri/Rust, and the small Python runtime supervisor.
- Never wipe/reset application data or a database, destroy existing records, start duplicate database servers, or replace working credentials. Ordinary additive migrations are allowed when required by the task; preserve backward compatibility.
- Never expose credentials in tracked files, logs, test output or chat. Use disposable state and fake credentials for checks.
- Be concise. Report what changed, meaningful checks and any real limitations. During substantial changes, use `/simplify` if available; otherwise make a focused manual simplification pass and say so honestly.

## Product boundaries

AI Deck manages temporary cloud inference for local coding projects. Read [README.md](README.md) for architecture and [docs/decisions.md](docs/decisions.md) for approved scope.

- No paid Runpod tests, resource creation, model downloads or paid inference are authorized by default. The owner will perform live acceptance. Do not turn on paid provisioning or invent total-budget/lifetime settings for testing.
- Preserve the configured hourly ceiling and explicit creation confirmation. Local timers are not independent offline cost protection. Retained storage remains billable after Pod deletion.
- Keep Provision, Enable and Open CLI separate. Closing a session/window must not silently terminate compute. Finish must preserve ownership checks, verify cloud deletion and wait for affected local process cleanup.
- Preserve saved deployment/session bindings. Catalog edits and new selections must not silently retarget existing sessions.
- AI hardware/context estimates, published model metadata, provider price snapshots and measured compatibility evidence are different data. Label them accurately. Never present mocked tests or generated suggestions as successful live model validation.
- The owner authorized automatic normal-subscription Codex analysis when importing a Hugging Face URL. Keep that narrow analysis integration separate from app-isolated coding terminals and normal terminal configuration.

## Rename compatibility

The project/package name is `ai-deck`, Rust library `ai_deck`, and displayed product **AI Deck**. Keep provider-specific Runpod names. The installed-app/Keychain identifier, saved Codex provider ID and remote ownership/runtime keys intentionally retain their earlier values to preserve saved data, histories and Pod cleanup. See [README project identity](README.md#project-identity). Changing these requires a deliberate compatibility design, not a global text replacement.

## Sources of truth

| Concern | Edit here |
| --- | --- |
| Native state, IPC types, policy defaults | Rust types and services under `src-tauri/src/` |
| Bundled model/runtime settings | `model-catalog/catalog.json`; validate in `model_catalog.rs` |
| Imported metadata, analysis and quotes | Existing `StateStore` through `model_import_service.rs` |
| Generated frontend types/defaults/catalog schema | Rust sources and `src-tauri/examples/export_contracts.rs`; run `npm run contracts` |
| Developer commands | `package.json` scripts; `Makefile` provides Mac-friendly wrappers |
| Shared UI state and command/event bridge | `src/shared/useDeck.ts` |
| CLI isolation and supported versions | `src-tauri/src/cli_adapters/` |
| GPU startup behavior | `containers/vllm-runtime/launcher.py` |

Do not hand-edit `src/shared/contracts.ts`, `src/shared/defaults.json` or `model-catalog/schema.json`. Do not edit build artifacts, dependency directories or generated Tauri schema files as implementation sources. Avoid introducing another store or a database dependency for the model library.

## Verification and documentation

- Keep Make targets thin: reuse existing npm scripts for shared command chains. `make doctor` is read-only; do not add implicit npm installation or automatic system-tool/CLI installation to other targets. Keep regular checks/tests free of cloud credentials and paid work.
- Add regression tests for changed behavior or a reproduced bug; avoid tests that merely repeat implementation details or test copy changes. Use existing fixtures and test infrastructure.
- After changing shared contracts/defaults/schema, run `npm run contracts` and inspect the generated changes.
- Run relevant tests, then `npm run check` and `npm test` for cross-layer changes. These cover generated drift, Vue/TypeScript build, Rust format/lints, native tests, Python runtime tests and Vue tests.
- Local listener/PTY failures may be environment permission problems. Report them accurately; never substitute real accounts or paid cloud resources to get a test passing.
- Optional ignored probes require deliberate selection. Inspect what a probe does before running it; preserve its fake credentials and disposable directories. A read-only public metadata fetch is not a successful model inference test.
- Rebuild the `.app` when delivering desktop changes. Do not open an app against the owner's saved cloud state merely to inspect layout; use an isolated fixture or browser preview.
- Keep README and feature docs aligned with actual behavior. Record verification limits in `docs/local-validation.md`; preserve historical audit evidence and distinguish it from current results.
