# Local validation record

Host: Apple Silicon Mac, macOS 26.6.2. Implementation work: 5–6 October 2026; release-readiness review: 8 October 2026; provider assessment/refactor: 10 October 2026. **No Runpod or GPUtw compute, retained storage, model downloads or paid inference were created by these checks.**

The latest **13 October 2026** Codex compatibility checks passed: generated-contract drift, production frontend, Rust formatting/Clippy and **106 regular tests** (61 Rust, 19 Python, 26 Vue). Four optional Rust probes are ignored by the regular suite; the two installed-Codex probes also passed separately. Earlier checks and their limits are preserved below.

## Codex version compatibility — 13 October 2026

The owner's Models screenshot showed analysis blocked by the CLI version gate. The installed Codex reports `codex-cli 0.162.0-alpha.17.2`, while the gate previously accepted only `codex-cli 0.160.0`. Checked the installed help and [official CLI reference](https://learn.chatgpt.com/docs/cli/reference), then ran the existing installed-binary probes before retaining the new exact version in the allowlist. Launcher and analyzer isolation flags are unchanged. Future versions remain blocked, and errors now show the detected version and accepted versions from the native gate. Setup copy no longer duplicates that version list.

| Check | Result and limits |
| --- | --- |
| `CARGO_NET_OFFLINE=true cargo test --manifest-path src-tauri/Cargo.toml --test installed_cli_smoke installed_codex -- --ignored --nocapture` | Both probes passed with Codex `0.162.0-alpha.17.2`. The analysis request included its strict JSON schema, restricted tool declarations and the disabled execution-host diagnostic. The session adapter reached localhost using its scoped fake credential; disposable normal authentication was unchanged. This is rejection-path validation, not successful subscription analysis or model inference. |
| `CARGO_NET_OFFLINE=true npm run check` | Passed: generated drift, Vue/TypeScript build, Rust formatting and Clippy with warnings denied. |
| `CARGO_NET_OFFLINE=true npm test` | Passed: 61 Rust, 19 Python and 26 Vue tests. Four opt-in probes ignored; the combined installed-session probe is now two independently selectable tests sharing the same implementation. Regressions accept the checked versions, reject a subsequent alpha and show useful version errors. |
| `CARGO_NET_OFFLINE=true npm run tauri -- build --bundles app` and `codesign --verify --deep --strict` | Passed. Rebuilt and verified the arm64 `src-tauri/target/release/bundle/macos/AI Deck.app` with its existing identifier and ad-hoc signature. The app was not opened against owner state; Developer ID signing/notarization was not performed. Replace any previously installed app copy to use this fix. |
| Other installed versions | Codex `0.160.0` and Claude Code `2.1.289` retain their earlier acceptance/evidence; their binaries were not rerun. Installed Claude Code now reports `2.1.296`, observed with `--version` only; it remains unverified and blocked. |
| Source and repository review | A focused manual simplification pass shared the existing routing probe between the separate CLI tests. Both probe listeners now explicitly set accepted sockets to blocking mode with the existing bounded read timeout, matching the shared HTTP fixture's macOS correction. `.gitignore` and `.gitattributes` already cover the outputs; no new rules or scoped agent instructions are needed. Local documentation link targets exist and `git diff --check` passes. No dependency, lockfile or generated-contract changes. |

The first sandboxed probe attempt was denied permission to bind localhost. Both probes and the regular suite passed with the authorized listener/PTY permission. No owner credentials/state were read, no CLI was installed/upgraded, no real subscription analysis or paid cloud operation ran, and no Git commit/push/merge was performed. A successful analysis through the owner's normal subscription remains an owner-led acceptance check.

## Provider setup and routing — 13 October 2026

### Follow-up contract review and transport correction

After the owner requested the full GPUtw provider, rechecked current vendor documentation and pinned reference source at `136fa0bf6bcd9870e2b860372db89931005d93ea`. Anonymous reads of public reference files, the empty public issue list and `/config/deploy` were made; no working Keychain/API key, account records, resource creation, storage write, image validation, model download or inference was used. The confirmed and unresolved contract details are recorded in [provider integration](provider-integration.md#contract-recheck--13-october-2026). The owner subsequently approved posting the prepared questions. [GPUtw-Skill issue #1](https://github.com/GPUtw-ai/GPUtw-Skill/issues/1) was created through the owner's signed-in browser after the connector returned a repository-access denial; the posted title/body/author and Open status were verified, and a screenshot was saved in ignored `.local/gputw-contract-review/posted-issue.png`. No vendor answer has been verified yet. The paid adapter remains unfinished; this is not a completion or live-readiness claim.

Corrected the native GPUtw client's missing `User-Agent`, which the vendor documents as required by its front door, and added explicit JSON `Accept`. Existing loopback fixtures now assert those headers. One focused test exposed the fixture's macOS accepted-socket nonblocking inheritance; the shared fixture now explicitly uses blocking accepted sockets with its existing bounded timeout. This corrects test transport rather than weakening production checks.

Final `CARGO_NET_OFFLINE=true npm run check` and `CARGO_NET_OFFLINE=true npm test` passed after both corrections: **106 tests** (61 Rust, 19 Python, 26 Vue), with the same three opt-in Rust probes ignored. The source change has no UI or shared-contract changes. The follow-up `CARGO_NET_OFFLINE=true npm run tauri -- build --bundles app` also passed, and `codesign --verify --deep --strict` passed on the rebuilt arm64 app. The app was not opened against owner state; this remains local ad-hoc packaging, without Developer ID signing/notarization.

Added visible Runpod/GPUtw selection, separate Keychain settings and read-only provider checks. Selection saves immediately; unrelated Setup drafts survive it. GPUtw creation remains gated in the interface and native service before any cloud request or creation intent. Provider identity defaults older records to Runpod and binds deployment lifecycle, endpoint routing and saved-price refresh. The selector does not change existing session/deployment snapshots. Read-only setup checks do not hold the deployment operations lock and delay cleanup.

| Check | Result |
| --- | --- |
| `npm run contracts` | Passed. Generated provider types/default, preview metadata and shared contracts from Rust. The generator now trims trailing line whitespace; generated files were not hand-edited. |
| `CARGO_NET_OFFLINE=true npm run check` | Passed on final source: generated drift, Vue/TypeScript production build, Rust formatting and Clippy with warnings denied. |
| `CARGO_NET_OFFLINE=true npm test` | Passed on final source: 61 Rust, 19 Python and 26 Vue tests; three opt-in Rust probes remain ignored. |
| Provider regressions | Passed: separate key storage and provider-specific active-deployment guards; old-record defaults; Runpod Enable/reconciliation/Finish after selecting GPUtw; unchanged saved fingerprint/endpoint; blocked GPUtw and stale creation requests; no unsupported-provider fallback; provider-bound imported quotes/refresh; key validation and unavailable-Keychain state; reads independent of the cleanup lock. |
| GPUtw HTTP fixtures | Passed against bounded loopback fixtures: only GET is sent, private records are discarded, invalid envelopes and authentication/redirect failures are rejected without returning upstream bodies. No working GPUtw key or account was used. |
| Vue regressions | Passed: immediate provider selection, draft-key clearing, preserved unrelated edits, provider-key command routing, honest read-access success, failure feedback, GPUtw deployment gate, cleared cached selection/confirmation and discarded late hardware results. |
| Isolated browser inspection | Inspected actual app components with disposable preview state at 1280 × 850 and 960 × 650. Both providers and GPUtw's key/readiness controls render; no horizontal document overflow was observed. GPUtw's Add API key action navigates to Setup, and readiness details expand. The confirmation-tab console had no warnings/errors. |
| Impeccable detector | One final mechanical scan returned no findings. Kept the existing dark workspace design and put detailed readiness requirements behind disclosure to make account setup easier to scan. |
| `CARGO_NET_OFFLINE=true npm run tauri -- build --bundles app` | Passed. Rebuilt `src-tauri/target/release/bundle/macos/AI Deck.app` for arm64 with the existing app identifier and ad-hoc signature. |
| `codesign --verify --deep --strict` | Passed on the rebuilt app. Developer ID signing/notarization and native interaction against owner state were not performed. |
| Repository hygiene | `git diff --check` passed. Generated `providers.json` is tracked/diffable and marked generated in `.gitattributes`; `.local/` already excludes the disposable preview and evidence. No dependency, lockfile or unrelated line-ending changes. |

The initial sandboxed full test attempt could not bind loopback listeners. The authorized offline suite passed with the necessary local listener/PTY permission. A shared HTTP fixture module was loaded twice during the first Clippy attempt; it is now registered once and reused by runtime and GPUtw tests. The final checks above passed after those corrections.

No `/simplify` command was available. The focused manual pass centralized provider metadata/key routing, kept old Runpod key commands as wrappers, shared the existing HTTP fixture, separated read access from paid eligibility and preserved saved-provider routing. No working Keychain entry or saved owner app state was read, no paid resource/model download/inference was used, and no Git mutation ran. The actual GPUtw read-access check remains owner-operated; startup, recovery, storage, paid lifecycle and live model acceptance are still open in [provider integration](provider-integration.md).

## Desktop UI refinement — 11 October 2026

Retained the existing dark workspace design and refined all five screens. Shared text is larger, secondary text has stronger contrast, navigation and feature icons reuse one SVG component, and long project/session names wrap at the supported minimum window width. Model facts remain separate from untested Codex estimates and dated provider quotes.

Setup and model-library drafts now survive screen navigation. Setup distinguishes unsaved changes from confirmed saves, preserves newer edits during a pending save, and retains the draft after failure. Executable fields have explicit accessible labels. Disconnected session screens/dialogs route users to Deployment, empty searches offer Clear search, and deployment status panels distinguish ready, neutral and failure states. Keyboard users can skip navigation; session status is announced independently of its colored dot.

| Check | Result |
| --- | --- |
| `CARGO_NET_OFFLINE=true npm run check` | Passed after the final UI source changes: generated drift, Vue/TypeScript production build, Rust formatting and Clippy. |
| `CARGO_NET_OFFLINE=true npm test` | Passed: 53 Rust, 19 Python and 20 Vue tests. New regressions cover settings feedback, normalization, failed/pending saves, disconnected-session routing and search recovery. |
| Isolated browser inspection | All five screens inspected at 1280 × 850 and the supported 960 × 650 minimum, including disposable ready/failed deployment data, long names, expanded imported-model facts, session history/terminal layout and the launch dialog. No horizontal document overflow was found in the inspected layouts. Draft navigation, Clear search, skip-link focus and Escape/focus restoration were exercised. |
| Rendered contrast and console | Final checks found no text-contrast failures in the rendered ready deployment, Setup or expanded Models views, excluding disabled controls. Fresh confirmation-tab console contained no warnings/errors. This was a focused manual check, not a full assistive-technology audit. |
| Impeccable detector | One requested manual scan returned no deterministic findings; the active edit hook also reported no findings. Rendered inspection supplied the visual evidence. |
| `CARGO_NET_OFFLINE=true npm run tauri -- build --bundles app` | Passed after the final UI changes: rebuilt `src-tauri/target/release/bundle/macos/AI Deck.app` for Apple Silicon, with the existing `com.runpoddeck.desktop` identifier. |
| `codesign --verify --deep --strict` | Passed on the final bundle. Tauri applied the existing local ad-hoc signature; Developer ID signing/notarization and native interactive acceptance were not performed. The app was not opened against owner state. |

The first full test attempt had one loopback HTTP fixture read timeout during concurrent app compilation. The isolated test and subsequent full suite passed without changing the native implementation or fixture. These checks use fake services, credentials and disposable state; no paid cloud operation, model download, real CLI session, saved owner state or Keychain access was used. Browser fixture source was temporary and excluded from the production entry point.

No `/simplify` command was available. The manual pass reused the existing shared styles, native bridge and compiler-based test harness, consolidated the icon paths, preserved view drafts without introducing another store, and kept native spending/ownership/session bindings authoritative. No dependency, toolchain, contract, lockfile or saved data was changed. Native interactive/live cloud acceptance remains owner-operated; browser terminal rendering does not prove a real coding session.

The **10 October 2026** full checks passed **84 regular tests** (53 Rust, 18 Python, 13 Vue), including two provider regressions and four wrapper-plan regressions. The later wrapper repository-label change passed **19 Python tests** and the built image passed the separate local supervisor/authentication check recorded next. The full native/UI suite was not repeated for that script-only change at that time. The initial 8 October review's 68-test result also remains historical evidence below.

## Verified registry upload — 13 October 2026

Resumed the owner-authorized GHCR publication. Current runtime sources and the catalog base matched the previously checked recipe; all 42 archive blob hashes matched. GHCR reused 41 blobs and accepted the remaining 5.15 GB layer after an 815-second direct HTTPS transfer. The client allowed 30 minutes instead of the earlier 570-second cutoff. A failed process-stop check briefly created a duplicate restart; both clients were then stopped and one uploader completed. No uploader remains running.

Authenticated registry readback verified the image's manifests, index and recipe-derived tag. Published identity: `ghcr.io/weiloon1234/ai-deck-runtime@sha256:fda0b0519a13eb0ccb9b76faa40908dc37c0a4a3eb87f82c646dc37a71bbde07`. Its Linux/amd64 runtime manifest/config match the isolated CPU supervisor/authentication fixture recorded earlier. No paid GPU resource, model download or inference was used.

Public visibility and association with `weiloon1234/ai-deck-runtime` are confirmed following fresh owner approval. Anonymous GHCR checks passed for the exact index digest, its single runnable Linux/amd64 manifest, the matching runtime config reference and the largest layer's digest/size via HEAD. No owner credentials were used by that verifier and no image layer was downloaded. The actual registry payload matches the earlier isolated CPU fixture; GPU/model inference remains untested.

The package page visibly shows Public and the selected linked repository. The initial automatic approval rejection was resolved, and no package setting remains pending. Documentation links and `git diff --check` passed. These were publication/package settings and documentation updates; no desktop implementation, app bundle, lockfile, saved app state or Git history was changed. No repository content or GitHub workflow was uploaded/executed. [Public package](https://github.com/weiloon1234/ai-deck-runtime/pkgs/container/ai-deck-runtime), [publication details](provider-integration.md#verified-registry-upload--13-october-2026).

Small verification records and the public-package screenshot are saved in ignored `.local/runtime-publication-2026-10-13/`. The task's 9 GB temporary OCI archive was removed after verification; the local Docker image and working GHCR login are retained.

## Runtime publication attempts — 11 October 2026

GHCR login and GitHub browser sign-in are complete. Registry metadata confirmed 41 of 42 required blobs, but the remaining 5.15 GB layer and image manifest are not published. Docker upload retries and the cached builder did not produce a verified registry pin. A direct HTTPS stream reached about 750 MB in nine minutes before its own 570-second deadline; the measured rate is below that needed for GHCR's documented 10-minute upload limit. No public visibility change or anonymous manifest verification has been completed.

The owner confirmed mobile-network use and explicitly paused publication until returning to fast home Wi-Fi. No upload client remains running. The verified local image/archive and existing GHCR login are preserved for that retry; the optional GitHub publishing bundle has not been executed.

The refreshed local image index is `sha256:fda0b0519a13eb0ccb9b76faa40908dc37c0a4a3eb87f82c646dc37a71bbde07`; the original runtime manifest/config remain unchanged. The same isolated CPU supervisor/authentication check passed again and fixture removal was verified. A 9,035,314,176-byte OCI export completed, and all 42 blob digests matched their local bytes before registry authentication.

Eight temporary offline resumable-upload fixtures passed, but the actual GHCR ranged-chunk/status responses did not confirm a supported recovery path. The prepared GitHub-hosted publication bundle passed five separate fake-service fixtures, source-hash validation, Python/shell syntax and ZIP integrity checks. These operational fixtures are separate from the 92-test desktop suite above. The bundle has not been committed, uploaded to its GitHub source repository or executed on a GitHub runner. The owner must perform that Git step; no agent Git commit/push/merge was run.

No GPUtw key/account, GPU resource, model download, paid inference or owner app state was used. The existing GHCR Docker credential helper was used solely for the explicitly authorized registry publication; no credential was printed or copied into the bundle. Desktop source/bundle and lockfiles were unchanged by this publication work. `.gitignore` and `.gitattributes` need no further changes because operational archives/helpers remain outside the repository. [Detailed attempts and prepared workflow](provider-integration.md#publication-attempts-and-prepared-github-workflow--11-october-2026).

## Wrapper build and local startup check — 10 October 2026

The owner selected `ghcr.io/weiloon1234/ai-deck-runtime`, linked to their `ai-deck-runtime` GitHub repository, with public visibility. The plan generator now accepts a validated optional `--source-repository` and includes that label in the recipe identity. `npm run test:runtime` passed **19 tests**: 8 supervisor, 6 read-only recovery-probe and 5 wrapper-plan cases, including label validation and identity changes.

The catalog-derived `buildArgv` was executed explicitly using the local Docker engine, `--platform linux/amd64`, `--network=none` for build execution and an isolated client configuration. The pinned public base layers were downloaded; the Dockerfile only copies the existing supervisor and clears the inherited command. No model files or packages were installed. The build passed. Its required base argument produced Docker's `InvalidDefaultArgInFrom` advisory; the actual argument was the validated catalog digest.

The resulting image was inspected for platform, entrypoint, empty command, absence of inherited volumes/health checks, recipe label and repository label. Its real entrypoint then ran in one temporary container with `--network none`, `--read-only`, 512 MiB memory, one CPU, 128 PID limit, dropped capabilities, no new privileges and a 64 MiB temporary filesystem. Fake model configuration and credentials were used, with no GPU or host project mounts. Authenticated `/status` responded; absent/wrong tokens returned 401 and an authenticated missing route returned 404. The image's launcher SHA-256 matched the source plan, and no ready inference result was reported. Container removal and absence were verified.

Local image tag: `ghcr.io/weiloon1234/ai-deck-runtime:ai-deck-811ed61312a2eddccefe0bfa`. Local image ID: `sha256:7db2ab1763ff2a01a77ebbc08eef2647fb49ef1000840ff02f509fccecf42447`. These identify the local build; they are not a verified published registry pin. Publication and anonymous manifest access were pending at this check; the later authenticated attempts are recorded above. No GPUtw account key, resource creation, GPU startup, model download or inference was used. [Build/publication details and remaining vendor contracts](provider-integration.md#selected-image-and-local-verification--10-october-2026).

The final isolated Python rerun passed all 19 cases. Documentation review resolved 76 relative file links, `git diff --check` passed, and Git attributes keep the Docker recipe/context filter, Python sources and new documentation as LF text. Existing `.gitignore` and `.gitattributes` require no further change. Native/UI sources and the rebuilt desktop bundle were unchanged by this image preparation.

## Final desktop checks and initial wrapper preparation — 10 October 2026

The owner explicitly authorized restoration of missing locked Rust dependencies. `cargo fetch --locked --manifest-path src-tauri/Cargo.toml` completed with an empty allowlisted environment and disposable home/temp state. Neither lockfile changed: Cargo SHA-256 `b29af03254a56fcaf6d4d938c3ec15350d6f6e4e186a9b8a08d39d3f59a1817a`; npm SHA-256 `0c07a352454dff24da6c7dee880d115fdf4048f36ffde3004df499a10e2a15c2`. No package version or toolchain was changed.

| Check | Current result |
| --- | --- |
| `npm run check` | Passed: generated files match Rust sources, Vue/TypeScript production build, Rust formatting and Clippy with warnings denied. |
| `npm test` | Passed: 53 Rust, 18 Python, 13 Vue tests; three opt-in probes ignored. The first sandbox attempt could not bind local HTTP listeners; the same suite passed with the approved loopback/PTY permission. |
| `npm run tauri -- build --bundles app` | Passed: rebuilt the current `src-tauri/target/release/bundle/macos/AI Deck.app` (arm64). Bundle name, version, executable and unchanged `com.runpoddeck.desktop` identifier were verified. The app was not launched against saved owner state. |
| `codesign --verify --deep --strict` | Passed on the final bundle. The first unsigned packaging attempt lacked a valid resource seal; Tauri now uses `signingIdentity: "-"` and the subsequent normal rebuild sealed the bundle automatically. This is local ad-hoc signing, not Developer ID signing or notarization. No signing account or Apple service was used. |
| GPUtw wrapper preparation | Added a minimal Dockerfile using the existing supervisor and a catalog-derived pinned base, plus a context filter and a read-only plan generator. Four offline tests verify catalog binding, invalid inputs, source/base identity changes and explicitly unbuilt/unpublished/disabled results. The real catalog plan printed successfully without invoking Docker or contacting a registry. |
| Repository/docs | Documentation links, whitespace and new-file LF attributes pass. Lockfiles and generated contracts/defaults/schema remain unchanged. Existing `.gitignore`/`.gitattributes` cover these files and the app output; the new Docker context filter excludes all files except its recipe and supervisor. |

These desktop checks after dependency restoration used offline Cargo, fake services/keys and disposable state. No saved app state, Keychain credential, GPUtw account, image pull/build/publication, model download or inference was used in this run; the later authorized local image build is recorded above. The initial cache blocker recorded below is resolved. The packaged app contains the UI cleanup and provider refactor; Intel packaging, native launch against owner state, Developer ID signing and notarization remain unverified. [Ad-hoc signing guidance](https://v2.tauri.app/distribute/sign/macos/#ad-hoc-signing). No `/simplify` command was available; the manual pass retained one supervisor and one base-image source, structured arguments and the existing test harness.

At this 10 October checkpoint, GPUtw remained unselectable: creation marker readback and persistence, authenticated startup health, provider-enforced storage quota and a published wrapper digest were unresolved. The 13 October publication above resolves image availability; the other contracts still block enabling GPUtw. No speculative adapter or paid operation was added. See [provider integration](provider-integration.md) for prepared diagnostics/recipe and the exact remaining contracts.

## GPUtw recovery contract probe — 10 October 2026

Added an opt-in, standard-library Python probe for marker readback from an existing owner-selected GPUtw record. Only fixed-host list/get requests are available; no create, mutation, inference or retry of creation is implemented. Keys require interactive hidden input, are not saved, and never appear in results. Redirects/proxies are disabled, bodies and structure are bounded, and failures are sanitized. The probe checks resource identity and consistent non-secret marker pairs, without mistaking an observation for idempotency or enabled production recovery. Source observations, invocation and the remaining vendor question are in [provider integration](provider-integration.md#checking-marker-readback-without-provisioning).

`npm run test:runtime` passed **14 tests** after the probe changes: the existing 8 supervisor cases plus 6 offline probe cases covering both readback shapes, missing/conflicting/duplicate matches, identity mismatch, transport methods/host, redirects/proxies, response limits, sanitized failures/output and terminal-only key input. Tests used fake records and fake keys; the authenticated probe itself was not run. No real key, cloud account state, paid operation or new dependency was used. Syntax/help, documentation links and `git diff --check` were also checked. A manual simplification pass kept transport, marker inspection and CLI input separate and reused the existing Python test harness.

GPUtw provisioning is still disabled. This diagnostic makes the missing contract testable; it does not resolve it without an appropriate existing record or vendor confirmation. The later owner-approved dependency restoration and native revalidation are recorded above.

## Provider assessment and refactor — 10 October 2026

Reviewed GPUtw's official API documentation, vendor reference schemas and read-only public GPU/deploy configuration endpoints. No authenticated API request or cloud mutation was made. The API fit, dated quotes, ownership correlation gap and supervisor-image requirements are in [provider-integration.md](provider-integration.md). Runpod remains the only selectable provider.

Moved shared resource/hardware types and the provider trait into `cloud_provider.rs`; moved bounded HTTP utilities into `http_client.rs`. Runpod owns payload construction and ownership checks, and application lifecycle services call those adapter hooks. Persisted state shapes, generated contract declarations, endpoint routing and Runpod ownership keys are unchanged. Request preparation now precedes inference-credential storage, so rejected preparation cannot leave an orphaned key. Two native regressions cover preparation rejection and adapter ownership denial across Enable, reconciliation and Finish; both passed in the final checks above.

| Initial check before cache restoration | Result at that point; superseded by final checks above |
| --- | --- |
| `npm run check` | Blocked during `contracts:check`: offline Cargo cannot resolve the locked `fs2` crate. The cache now contains some crates, but remains incomplete. No Rust compilation or Clippy result is claimed for this refactor. |
| `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` | Passed. |
| `npm run build` | Passed: Vue/TypeScript and production frontend. |
| `npm run test:ui` | Passed: 13 tests. |
| `npm run test:runtime` | Passed: 8 tests. |
| Full native tests / generated contracts / desktop packaging | Pending the earlier owner-approval request to restore existing locked crates. No package, lockfile or toolchain was changed; the existing app bundle predates the UI cleanup and provider refactor. |
| Repository/source review | Import references and all production ownership call sites reviewed. README, native/root AGENTS and provider decisions updated; local documentation links resolve and `git diff --check` passes. No new ignored output/file type was introduced; `.gitignore` and `.gitattributes` remain suitable. |

Initial checks used a disposable home/temp directory, an allowlisted empty environment and offline Cargo. No working credentials or saved application state were used. No `/simplify` command was available; the manual pass retained the existing adapter and fixtures, removed Runpod imports from generic clients and reused one payload builder/ownership implementation. Later final-check results are recorded above.

## UI cleanup — 8 October 2026

Simplified page headings, navigation icons, spacing, typography and shared surfaces. Empty deployment telemetry is hidden until a Pod exists; active telemetry uses one grouped surface. Imported model facts, hardware and quotes remain accessible in expandable details, with a stable library total and search. Ended sessions are available through a sidebar disclosure. Setup's save controls stay visible while scrolling, and navigation returns to the top of the new screen. Provision, Enable and Open CLI remain separate; creation limits, confirmations, candidate labels, pricing caveats and native command gates are preserved.

| Check | Result and limits |
| --- | --- |
| `npm run test:ui` | Passed: 13 tests. Added regressions for library search/count deduplication, deployment telemetry and workflow gates, session history access and navigation scroll reset. |
| `npm run build` | Passed: Vue/TypeScript checking and production Vite build. |
| Browser layout and interaction | Inspected at 960×650 and 1280×850 using disposable model/project/session/deployment fixtures. Checked expanded model details, project rows, ended-session access, the terminal surface, session dialog and Setup save controls. No horizontal document overflow or console warnings/errors were observed. The fixtures were removed after review and are absent from the production build. |
| `git diff --check` | Passed. Shared style selectors reviewed for accidental removal. |
| Native checks and desktop packaging | Initially blocked: offline Cargo could not resolve `async-trait` because the registry cache was empty. Automatic approval review rejected restoration without explicit owner approval. The owner approved restoration on 10 October; later final results are recorded above. No dependency version was changed. |

No working state, credentials, cloud calls or paid resources were used. Browser preview retains its native-command boundary. No `/simplify` command was available; the manual pass reused existing model components, retained shared styles in one file and extracted the existing VNode text helper for both UI test files.

## Release-readiness review — 8 October 2026

Reviewed the Vue command/event flow, native lifecycle and provider/runtime clients, saved state and credentials, terminal/process ownership, CLI adapters, model import/analyzer boundaries, GPU supervisor, build commands and repository configuration. This was a release-readiness review with security source checks, not a full security audit or live model validation. Findings were reviewed independently after correction.

| Reproduced issue | Correction and regression |
| --- | --- |
| The pinned Hugging Face config link was rendered but rejected by the native URL allowlist | The native validator accepts only the immutable `org/repo/blob/<40-hex-revision>/config.json` shape alongside existing provider pages. Tests retain host, scheme, credential, port, query and path restrictions without opening a browser. |
| Single-session imported profiles required impossible two-session compatibility evidence | The concurrent-session check applies only to profiles allowing multiple sessions. Regressions keep core coding/isolation checks and Claude helper checks mandatory. No live evidence was added. |
| Decodable saved state with a malformed installation ID could reach unchecked slicing during provisioning | Startup validates the generated hyphenated UUID format before rewriting state. Regressions preserve malformed original bytes and valid identity spelling/case. |
| A repeated non-ready HTTP response could bypass the supervisor loading deadline | Every unsuccessful readiness poll now checks the deadline. The offline regression returns HTTP 204 and verifies sanitized timeout status, graceful/forced process-group cleanup and status-server shutdown. |

Current checks:

| Command or inspection | Result |
| --- | --- |
| `make doctor` | Passed with the existing Node/npm/Rust/Python toolchain. Isolated-home Xcode cache warnings were nonfatal. |
| `npm run check` | Passed after the final source changes: generated drift, Vue/TypeScript build, Rust formatting and Clippy with warnings denied. |
| `npm test` | Passed: 51 Rust, 8 Python and 9 UI tests; three deliberate opt-in probes ignored. |
| `npm run tauri -- build --bundles app` | Passed: rebuilt `src-tauri/target/release/bundle/macos/AI Deck.app` for Apple Silicon. Bundle identifier remains `com.runpoddeck.desktop`. The app was not opened against the owner's saved state. |
| `npm audit --package-lock-only --ignore-scripts --json` | Public npm advisory service reported zero known vulnerabilities across 85 locked dependency entries. No dependency or lockfile was changed. |
| `.gitignore` / `.gitattributes` | Reviewed with `git check-ignore`, `git check-attr` and tracked line-ending inspection. Build outputs, secrets and diagnostics are ignored; lockfiles/generated sources remain tracked; text is LF and icons are binary. No change was needed. |
| README / AGENTS links and final diff | Local links resolve; `git diff --check` passes. Root and existing guides updated; focused guides added for catalog, coding adapters, native/UI/runtime tests and docs. |

Checks used an empty allowlisted process environment, disposable home/temp state, fake test credentials and offline Cargo. The owner explicitly approved execution under the available Codex sandbox instead of the security skill's stricter separate sandbox requirement. The first Rust attempt failed because local listeners were denied; the same regular suite passed after granting loopback/PTY permission. The npm registry was contacted only for the separately approved advisory lookup; no source or working credentials were sent.

No Cargo advisory scanner is installed, so the Rust dependency advisory database was not scanned. The npm result is a dated advisory result for npm dependencies, not proof that the whole application is vulnerability-free. Live Runpod creation/deletion, effective endpoint protection, model startup and coding compatibility remain unverified. The supervisor edit changes fingerprints for newly resolved profiles; existing deployment snapshots are preserved and no compatibility evidence was copied.

No `/simplify` command was available. A manual pass kept URL validation separate from browser launch, reused the saved-state corruption error, retained one required-check list with a capacity condition and moved deadline enforcement to the existing poll loop. No package, toolchain, working account state or credential was changed.

## Earlier implementation checks — 5–6 October 2026

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
