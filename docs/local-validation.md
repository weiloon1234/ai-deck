# Local validation record

Host: Apple Silicon Mac, macOS 26.6.2. Implementation work: 5–6 October 2026. **No Runpod Pods, network volumes, model downloads or paid inference were created by these checks.**

The current regular suite passes **47 Rust tests, 7 Python tests and 9 Vue component tests** (63 total). Build, generated contracts, formatting and Clippy checks pass. A new installed-Codex analysis probe and a read-only public Hugging Face metadata probe passed separately. The earlier installed-CLI session test passed for both installed binaries during the original implementation and was not rerun for this feature.

**Mac Makefile workflow (6 October):** added a GNU Make 3.81-compatible command interface and colleague setup guide. `make doctor`, default help, missing-dependency guidance, `make -j4 verify` (all 63 regular tests) and `make build` passed on this Mac. A Make/npm jobserver warning found during verification was resolved by keeping Make jobserver settings out of npm child processes; `make -j4 check` then passed without those warnings. Setup and clean were inspected with dry runs only; no npm reinstall, crate prefetch or cleanup was performed. A fresh colleague Mac and Intel packaging remain unverified. Existing npm commands remain authoritative, with the Python test command shared through `test:runtime`.

**AI Deck rename (6 October):** project moved to `~/Projects/ai-deck`; npm/Cargo package and Rust crate, visible app/window/terminal copy, icon, diagnostics filename and new Pod name prefix now use AI Deck naming. Existing app-data/Keychain identity, Codex provider ID and remote ownership/runtime keys remain stable for compatibility. Generated Tauri caches containing the old absolute project path were rebuilt. Contracts, production frontend, formatting, Clippy and all 63 regular tests passed after the rename. No account state or credentials were migrated, no paid tests ran, and Git initialization/commits/pushing remain owner-managed.

**Model library (6 October):** URL import, automatic subscription-analysis integration, hardware/context suggestions, native price calculation, local persistence/reuse, independent price refresh and cancellation are implemented. Fake analyzers/providers and a fake CLI verify successful saving and failure paths; the installed Codex probe confirms accepted flags, JSON schema and restricted tool declarations with a disposable home and localhost rejection server. Public metadata/config/card retrieval passed against Hugging Face. The browser Models screen was visually inspected. No real subscription inference, account-specific Runpod price query, or paid GPU run was performed. See [model import](model-import.md).

**Model details and contributor guidance (6 October):** imported models display saved Hugging Face facts beside the shared Codex GPU/context suggestion in both Models and deployment preparation. Tests cover published versus suggested context, nested language configs, missing/malformed values, old saved metadata, bounded parameter counts and selection changes. The public metadata probe also verified parameter count and license retrieval. Build, generated-file checks, formatting/lints and the full local suite passed. The expanded specifications and selection flow were inspected in a browser using disposable fixtures. README now documents the architecture; root, frontend, native and runtime agent guides document existing development boundaries. No dependency or live account state was changed.

**Re-audit fixes (6 October):** all seven reproduced findings are corrected. Regressions exercise detached children after Close and parent exit, unrelated-session survival, transient endpoint recovery, startup-history handling, restored idle deadlines, legacy snapshots, damaged catalog/Keychain recovery, project-row selection and insufficient cache capacity/free space. See the [implementation re-audit](implementation-re-audit.md) for the original findings and resolution table. Cloud acceptance remains unverified.

## Scope and results

| Check | Result and limits |
| --- | --- |
| Vue / TypeScript production build | Passed. TypeScript was pinned to 5.9.3 after the Vue checker rejected the initially selected TypeScript 7 package layout. |
| Rust lint checks | Passed with warnings treated as errors. |
| Controller, catalog and storage tests | Passed with in-memory cloud/credential services and disposable durable state. Covers separate Provision/Enable, disabled spending, required limits, lost create/delete responses, ownership mismatch, actual-rate reconciliation, retained storage, startup failure, timeout/deadline cleanup, route invalidation, profile snapshots, restart revalidation, late creation and storage-write failures. |
| Local HTTP verification | Passed against loopback fixtures. Bad authentication must be rejected; wrong models and failed inference results cannot become Ready. Upstream error bodies are not surfaced. |
| Real local pseudo-terminals | Passed: input/approval-like prompt, Unicode, paths with spaces, resize, interrupt, process cleanup, output replay sequencing and secret redaction across chunk boundaries. |
| Two fixture coding sessions | Passed: distinct project folders/configuration histories, capacity enforcement, close without Pod deletion, idle countdown, and resume with original binding despite a changed enabled selection. These are fixture CLIs, not model coding results. |
| Runtime Python checks | Passed with mocked Hugging Face APIs. Protected-file access is checked before download; pinned revision/cache/token settings are passed; access failures and missing weights prevent download; output contains only sanitized error categories; status requires authentication. |
| Installed CLI rejection probes | Passed separately for Codex 0.160.0 and Claude Code 2.1.289. Actual installed binaries reached a localhost rejection gateway with app-scoped fake credentials. A disposable normal credential file was unchanged. This validates argument parsing/routing for the error path, not successful inference or a full real-subscription isolation test. |
| macOS app packaging | An Apple Silicon `.app` bundle was produced and opened on this host. Personal build; no distribution signing/notarization. |
| Native interface smoke check | Deployment screen renders; paid provisioning and availability are disabled without setup; USD 10/hour ceiling and blank total/lifetime are visible; both installed CLIs detected; new-session dialog blocks launch without Enable. |
| Browser layout check | Deployment, models and setup render. Browser preview is labeled and native controls cannot operate there. |

Run the current checks using the [README commands](../README.md). The opt-in installed-CLI probe is excluded from ordinary `npm test` because other developers may not have those exact installations. All cloud calls in controller tests are mocked. Tests do not read working account keys, change normal CLI configuration, reset databases, or perform Git mutations.

## Simplification review

No `/simplify` skill/command was available. Manual review centralized recovered-Pod price enforcement, retained one catalog and one enabled-route record, reused adapter settings at both config-file and command-line precedence, removed unused direct dependencies, generated frontend contracts/defaults from Rust, and kept diagnostics to an explicit allowlist. The follow-up review centralized idle-deadline calculation and terminal cleanup waiting, kept process ownership in a focused macOS helper, and shared the cache requirement across catalog validation, UI, provisioner and runtime. Terminal shutdown now observes descendant process identities and drains output before allowing resume; uncertainty prevents successful shutdown.

## Remaining live gates

- A successful model-library analysis using the owner's normal Codex ChatGPT subscription and account-specific Runpod GPU price feed. Public Hugging Face retrieval and offline Codex request construction have been checked separately.

- A successful Codex coding task and follow-up against the pinned Runpod runtime/model.
- The equivalent experimental Claude task, including helper requests.
- Side-by-side normal subscription routing with real CLI history and project overrides.
- Actual Docker startup, GPU compatibility, model download/cache reuse, protected repository access, latency and concurrency.
- Real Runpod creation/deletion and recovery under network interruption, sleep and crashes.
- Verification that actual HTTP inference/status endpoints are protected by the pinned image and exposed as expected by Runpod.
- A demonstrated independent expiry mechanism before any claim of offline cost protection.
- Native terminal copy/paste and complete interactive CLI approval/resume flows during the owner-led coding test.

The complete cloud acceptance matrix is still open. Refer to [model-compatibility.md](model-compatibility.md) and the [project plan](../runpod-coding-desktop-project-plan.md). The implementation checkmarks in the plan must not be read as a production-readiness or tested-model claim.

The model-import simplification pass kept schema generation in Rust, reused existing durable storage and provider quoting, gave imports a separate lock so cloud cleanup remains available, and extracted the Vue component fixture shared by old and new UI regressions. No dependency was added.

The model-details simplification pass reused the saved config instead of storing derived facts twice, and extracted one shared GPU-suggestion component for both screens. Agent guidance is scoped by component responsibility; generated-file ownership is documented at the root. No `/simplify` command was available, so this was a manual review.
