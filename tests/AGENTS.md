# UI and runtime test guidance

Inherits [root guidance](../AGENTS.md). Native tests have [their own guide](../src-tauri/tests/AGENTS.md).

- UI tests use Node's built-in test runner and `ui/component_fixture.cjs` with existing Vue compiler packages. Extend that fixture instead of adding a framework, dependencies or a second implementation of the native bridge.
- Test user-visible facts and event flows: selection, commands, missing data, published versus estimated values and error/busy behavior. A rendered link or disabled button does not prove native validation; add a native regression when that boundary changes.
- Runtime tests use Python `unittest` and the standard library. Mock Hugging Face, downloads, subprocesses, status listeners and time for supervisor lifecycle cases. Do not pull an image, fetch weights or start a real runtime.
- Read-only diagnostic probes under `scripts/` also use offline `unittest` fixtures here. Mock transport and interactive credential input; cover ownership ambiguity, fixed-host routing, response limits and sanitized output. Never invoke the authenticated probe through the regular test command.
- Preserve global launcher state between cases with context-managed patches. Test deadline and child cleanup behavior without long real sleeps or leftover listeners/processes.
- Keep all fixture state disposable, credentials fake and outputs sanitized. No working API keys, normal CLI state or app-data directories are needed.

Run `npm run test:ui` or `npm run test:runtime` for focused changes. Source changes crossing Rust, Vue or the supervisor also need the root checks and rebuilt desktop bundle.
