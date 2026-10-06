# Implementation re-audit

Date: 6 October 2026. Scope: compare the current application with the project plan, inspect implementation paths, rerun local checks, and reproduce failure cases without paid resources.

**Follow-up: the seven reproduced findings are now corrected, with passing local regressions and a rebuilt personal macOS app.** This does not pass the cloud acceptance gates or establish either model's coding compatibility. No Runpod resources, model downloads, account requests or paid inference were used in the audit or fixes.

## Resolution and current evidence

The evidence and bundle hash in this section record the audit-fix build. A later model-library feature build supersedes that binary; see [current validation](local-validation.md) and [model import](model-import.md). The seven fixes below remain covered by the current suite.

| Finding | Correction | Regression evidence |
| --- | --- | --- |
| F1 | macOS supervision tracks unique process/parent identities across separate process groups and reparenting; shutdown waits for observed descendants. Finish confirms cloud deletion and also waits for local cleanup before returning success. Uncertain cleanup prevents successful Quit. | Real PTY fixture starts a detached child that ignores TERM and continues writing. Close and natural parent exit both stop it, a second session keeps writing, and final shutdown stops that session too. Synthetic ancestry test rejects reused PIDs. Fixture session test checks Finish waits for termination. |
| F2 | Only a successful invalid-auth response establishes absent protection. Rate limits, redirects and gateway errors remain unconfirmed, block launches and permit later recovery without deleting the Pod. Initial startup still has a deadline. | Loopback 302/429/502/503/504 checks plus controller failure → blocked launch → recovery checks. Existing real-auth and wrong-model rejection tests remain passing. |
| F3 | Durable startup history separates initial startup from later outages. Legacy unknown history is handled conservatively. Lifetime/budget expiry still applies during recovery. | Initial, previously Ready and unknown-history cases; recovery after status failure; lifetime expiry afterward. |
| F4 | One shared idle-deadline function handles normal exit and restart. It never extends an earlier deadline. | Reopen with an abandoned running session, fresh countdown, earlier expired countdown and repeated restart. |
| F5 | Project ID travels from the clicked row through App into the launch dialog. Generic new-session actions reset that selection. | Two compiled Vue component tests exercise the actual event handlers and dialog setup. |
| F6 | Durable state is returned independently from catalog/credential availability. Explicit errors accompany empty catalog/unknown key status, while deployment and Setup controls remain available. | Denied fake credential store and damaged catalog together; saved deployment remains visible; Finish works after credential recovery even before catalog repair. |
| F7 | Catalog `minCacheGb` is shared by schema validation, UI, pre-creation volume checks and runtime free-space checks. Complete current-revision files are credited once; unrelated revisions and incomplete downloads are not. | 1 GB volume rejected before creation; boundary-size volume accepted; runtime blocks insufficient/unreadable storage; cached-file/deduplication checks. |

Current checks: **35 Rust + 7 Python + 2 Vue tests passed**. `npm run check` passed generated-contract verification, production frontend build, formatting and Clippy. `npm run tauri -- build --bundles app` produced the updated Apple Silicon app. The opt-in installed-CLI rejection probe was not rerun; no CLI adapter configuration changed.

Final process-error hardening was followed by another passing PTY suite, Clippy check and app rebuild. The packaged executable matches the release executable: 11,747,088 bytes, SHA-256 `127c270fc7e22ebb1beaaaacb3759c85423411df0607f6cda4faa95426a9bd14`. Reopen the app to use the rebuilt binary; this follow-up did not launch it against the owner's saved cloud state.

The runtime/profile fingerprint changed. Bundled profiles were versioned forward; neither has live validation evidence. Legacy saved profiles remain readable and retain their original fingerprints. Older imported catalogs must add the storage requirement before they can provision again; this does not hide existing deployment recovery.

Process supervision uses macOS kernel process identities, not environment variables, command-line secrets or negative-PID group signaling. Apple's [process metadata layout](https://github.com/apple-oss-distributions/xnu/blob/main/bsd/sys/proc_info_private.h) and [persistent original-parent identity](https://github.com/apple-oss-distributions/xnu/blob/main/bsd/sys/proc_internal.h) informed the implementation. It is an in-app cleanup mechanism, not an OS security boundary: it cannot guarantee containment of deliberately daemonized tools whose intermediate ancestry disappears before observation, or cleanup after the app is killed. Other platforms remain deferred.

The remainder of this document preserves the **original pre-fix audit** and its then-current source line references, counts and bundle hash. Its open-finding statements describe that earlier build; use the resolution table above for current finding status. Live acceptance gaps in the original audit still apply.

## Original findings

### F1 · P1 · Closing a session can leave its tool processes editing files

`TerminalManager` signals the original CLI process group. A child that starts another process group/session escapes those signals. After the parent exits, the manager can mark the terminal stopped and report successful shutdown while that tool remains active.

**Reproduced:** launched a disposable Python parent through the real local PTY; its child used `start_new_session=True` and periodically wrote a heartbeat in the temporary project. After `shutdown()`, the result was success and `is_running()` was false, but the heartbeat continued changing. The probe explicitly killed its own child afterward. This proves continued execution, rather than merely observing an unreaped PID.

**Impact:** Close Session, Finish, and Quit cannot currently promise that all affected local commands stopped. A background command may continue modifying the project after the UI says the session ended.

**Correction:** track and terminate the owned descendants, including separate process groups, and wait for verified cleanup before claiming completion. Avoid signaling unrelated or reused PIDs. Add a regression with a child that changes process groups and keeps writing after the parent exits.

Source: [terminal_process.rs](../src-tauri/src/terminal_process.rs), lines 117–129 and 224–250. Related plan: Phases 5 and 6, process state and Finish behavior.

### F2 · P1 · Temporary HTTP errors can cause automatic Pod deletion

The invalid-credential probe treats every HTTP response other than 401/403 as proof that endpoint authentication is absent. The reconciler treats the resulting `endpoint_not_protected` error as fatal and terminates the Pod.

**Reproduced:** a loopback server returned 429, 502, 503, and 504 separately. All four became `endpoint_not_protected`. A second controller probe fed that error into verification and observed deletion of the in-memory Pod and a final Terminated state.

**Impact:** rate limiting or a temporary proxy failure during startup or periodic verification can destroy an otherwise usable deployment, interrupt sessions, and force another billed startup.

**Correction:** distinguish an accepted invalid credential from an unavailable or inconclusive endpoint. Keep new launches blocked while verification is uncertain, retry transient failures within policy, and retain genuine authentication/model mismatch handling.

Source: [runtime_client.rs](../src-tauri/src/runtime_client.rs), lines 86–100; [deployment_reconciler.rs](../src-tauri/src/deployment_reconciler.rs), lines 158–174. Related plan: Phases 2, 3 and 6, verification and recovery.

### F3 · P2 · A status outage after successful startup is treated as startup timeout

On any runtime-status error, the reconciler compares the current time with the deployment's original creation time. It does this even after the deployment reached Ready. There is no durable distinction between initial startup and recovery from a later outage.

**Reproduced:** created a Ready fixture deployment, moved its creation time beyond the configured download + load + 300-second startup allowance, kept its lifetime deadline in the future and budget sufficient, then injected one status network error. The controller deleted the Pod with “Startup exceeded the bounded wait.”

**Correction:** preserve successful-startup history and apply startup deadlines only to initial startup. Handle later connectivity loss with the explicit recovery policy; do not infer startup failure from total deployment age.

Source: [deployment_reconciler.rs](../src-tauri/src/deployment_reconciler.rs), lines 189–202. Related plan: Phase 6, reconnect and sleep recovery.

### F4 · P2 · Restart does not restore idle cleanup after ending stale sessions

`StateStore::open()` marks saved running/starting/disconnected sessions Ended, but does not arm the deployment's idle cleanup deadline. The normal process-exit callback contains that logic and is not invoked on restart.

**Reproduced:** saved a deployment with one running session, a 30-second idle policy, and no pending idle deadline. After closing and reopening the disposable state store, the session was Ended and `cleanup_due_at` remained absent.

**Impact:** after a crash and restart, a now-idle Pod can keep billing until its full lifetime/budget deadline or manual Finish, rather than the configured idle delay. This is separate from the already disclosed lack of offline expiry.

**Correction:** reconcile session endings and idle deadlines together on startup, preserving earlier deadlines and scheduling cleanup when the last recorded active session is ended.

Source: [state_store.rs](../src-tauri/src/state_store.rs), lines 56–67; [session_service.rs](../src-tauri/src/session_service.rs), `session_exited`. Related plan: Phase 6, crash recovery and idle cleanup.

### F5 · P2 · A project's Open CLI button selects the first project instead

Every project row emits the same `launch` event without a project ID. The app opens a generic dialog, whose initial project is always `projects[0]`.

**Reproduced:** compiled the actual Vue project component with the installed compiler and supplied two fixture projects. Clicking the second row emitted only `["launch"]`; executing the actual dialog setup selected `project-a`. No native command or network action was allowed by the probe.

**Impact:** accepting the dialog without noticing its changed folder starts the CLI in another project, where later edits and commands run.

**Correction:** pass the clicked project ID through the app into the dialog; reserve the generic default for generic New Session actions.

Source: [ProjectsView.vue](../src/features/projects/ProjectsView.vue), lines 3 and 9; [App.vue](../src/App.vue), ProjectsView event handler; [LaunchSessionDialog.vue](../src/features/sessions/LaunchSessionDialog.vue), line 9. Related plan: Phases 4–5, project selection and sessions.

### F6 · P2 · A catalog or credential-store failure hides all saved deployment state

The snapshot is all-or-nothing: reading the catalog and checking Keychain must both succeed before the UI receives saved deployment/session metadata. On initial load, either failure leaves `snapshot` null. The app then has no current-deployment card or ordinary Finish/console recovery controls, and Setup has no form because settings were never returned.

**Reproduced:** a valid saved Ready deployment remained readable through `deployment()`, while an injected credential-store denial made `snapshot()` fail. A damaged disposable catalog also made `snapshot()` fail even though the backend could still Finish that deployment successfully using its stored profile snapshot.

**Correction:** return durable state independently from catalog/credential availability, with explicit status/errors for each auxiliary service. Keep deployment identity, billing uncertainty, recovery links and appropriate cleanup controls visible.

Source: [app_core.rs](../src-tauri/src/app_core.rs), lines 36–43; [useDeck.ts](../src/shared/useDeck.ts), `refresh`; [SetupView.vue](../src/features/setup/SetupView.vue), `v-if="form"`. Related plan: Phases 4 and 6, useful recovery actions.

### F7 · P2 · Retained cache capacity is never checked

Provisioning validates the network volume's identity and data center but ignores its size. The profile has container disk size and cache paths, but lacks the planned minimum cache/free-space requirement. The runtime also does not check free space before downloading.

**Reproduced:** the provider fixture offered a 1 GB retained volume. Provisioning the first bundled model accepted it and sent the create request. The cache paths resolve inside the attached volume; the separate container disk allocation does not enlarge that cache volume.

**Impact:** a clearly inadequate retained cache can start billed compute before model download inevitably runs out of space. Existing used space can also make a nominally large volume inadequate.

**Correction:** define the storage requirement once in the catalog/schema, reject known-insufficient capacity before creation, and verify usable free space inside the Pod before a large download. Reuse the same requirement in UI and runtime validation.

Source: [deployment_service.rs](../src-tauri/src/deployment_service.rs), lines 386–396 and `create_payload`; [model_catalog.rs](../src-tauri/src/model_catalog.rs), `ModelProfile`; [launcher.py](../containers/vllm-runtime/launcher.py), `download`. Related plan: section 5 storage fields; Phases 2–3.

## What is implemented

These entries describe code that exists. They do not certify live cloud behavior.

| Area | Implemented behavior | Evidence and limits |
| --- | --- | --- |
| macOS desktop | Tauri/Vue app, deployment/models/projects/sessions/setup screens, personal Apple Silicon bundle | Build passes; prior native smoke record. F5/F6 affect workflows. No distribution signing/notarization. |
| Catalog and contracts | One Rust schema, generated frontend contracts/defaults, JSON import, pinned model revisions/image, profile fingerprint and stored deployment snapshot | Catalog/storage tests pass. Two candidate profiles, zero live validated combinations. Missing storage requirement: F7. |
| Runtime launcher | Structured Python entrypoint, protected-file preflight, revision download, cache paths, authenticated status, bounded worker/startup supervision, sanitized error categories | Five mocked Python tests pass. No image startup, GPU, real download, cache reuse or successful model inference tested. |
| Provision/Enable | Separate operations; one active deployment; one enabled route; Ready requires invalid-auth rejection, model identity and inference result | Local lifecycle and HTTP checks pass on covered cases. F2/F3 affect verification/recovery. Real provider behavior remains open. |
| Cloud lifecycle | Durable creation intent, no automatic POST retry, ownership markers, lost-create recovery, repeatable deletion and absence confirmation | Fourteen lifecycle tests pass using an in-memory provider. Late creations, duplicate cleanup and actual-rate rejection are exercised. |
| Credentials | OS credential-store adapter, separate inference token, HF secret reference, controlled child environments, redacted terminal output, allowlisted diagnostics | Local fake-credential/config tests pass. Keychain failure degrades the whole UI: F6. Real subscription isolation is unproven. |
| Codex adapter | Executable/version detection, app-owned CODEX_HOME, explicit Responses route and credential, launch/resume arguments, normal approvals | Fixture tests and earlier installed-binary rejection probe. No successful real model coding loop. |
| Claude adapter | Separate app-owned configuration, explicit Messages route/authentication, helper model mappings, disable option, experimental labeling | Fixture checks and earlier installed-binary rejection probe. Native Messages success, helpers and coding loop unverified. |
| Sessions | PTY input/output, replay, resize, Unicode, redaction, naming/switching, capacity limit, per-session project/state, original-binding resume | Real local PTY and two fixture CLI tests pass for covered cases. F1/F5 affect cleanup/routing. Native clipboard and real approvals/resume remain open. |
| Cost and recovery | Spending disabled by default, USD 10/hour ceiling, required budget/lifetime, actual-price check, local deadline, idle delay, cost estimate, quit choices | F3/F4/F6 affect recovery. No independent expiry; retained storage has warnings, not a calculated storage-price total. |

## Plan coverage

All ten phases were compared with the implementation and existing evidence. The owner-approved change in order allowed local implementation before cloud testing; it did not waive live exit gates.

| Phase | Audit disposition |
| --- | --- |
| 0 · prerequisites | Stack, scope, tools, candidate revisions and hourly ceiling settled. Actual access/GPU/region availability, total experiment budget, lifetime and independent cleanup remain unresolved for a paid run. |
| 1 · full coding loop | Not performed. Every live-loop checkbox and the exit gate remain open. The installed-CLI 401 probe is only partial evidence for routing/error behavior. |
| 2 · runtime/catalog | Implemented with F2/F7 gaps. Two candidates share a pinned image; no tested profile pair and no live exit-gate pass. |
| 3 · provisioning | Native client/state machine and mock lifecycle coverage present. F2/F7 need correction. API wire behavior, volume attachment and full lifecycle demonstration still need real validation. |
| 4 · desktop/launchers | Local desktop and adapters present. F5/F6 remain. Full subscription/project-override/daemon isolation gate is open. |
| 5 · terminals/sessions | PTYs, session list, metadata and fixture concurrency present. F1/F5 remain. Actual clipboard, CLI approval/resume and tested model concurrency are incomplete. |
| 6 · recovery/release | Personal bundle and local controls exist. F1–F4/F6 prevent a complete recovery claim. Offline expiry is explicitly unsupported; cloud release matrix is open. |
| 7 · Claude | Experimental adapter exists. Successful Messages/tool/helper behavior and real isolation tests remain open. No combination is labeled validated. |
| 8 · optional expansion | Deferred by approved scope. Catalog import exists; custom chat, search, new runtime families and multiple active Pods are not delivered. |
| 9 · Windows | Deferred. No Windows execution decision, tested process cleanup, UI/link handling or release package. A portable dependency is not Windows delivery. |

Other plan details: model-specific supported storage modes/minimum free space are incomplete (F7); explicit cache deletion is not implemented in-app; separately managed volumes must be managed externally. Per-stage startup/download durations and inference latency are not recorded—the deployment record stores overall timestamps and a cost estimate. No protocol bridge was added because no live translation gap has been demonstrated.

## Acceptance matrix audit

This table maps every scenario in plan section 11. “Local” means the stated behavior has only the listed local evidence, not that its end-to-end cloud acceptance gate passed.

| Scenario | Current evidence / remaining work |
| --- | --- |
| Public model, no cache | Runtime/payload code and mocks only. Actual download/load/authenticated coding loop open. |
| Gated model, valid token | Secret-reference payload and mocked protected-file check. Real gated download open. |
| Missing approval or invalid token | Mocked 401/403/404 mapping and lifecycle failure cleanup. Actual provider/Hub case open. |
| Cached model revision | Explicit revision cache paths; no real reuse/incomplete-download test. F7 affects capacity. |
| Different tested model | Two candidate profiles, no tested alternate model. |
| Ready Pod before Enable | Local test confirms Ready does not select a route. Real Pod test open. |
| Enable then Open CLI | Fixture launch uses selected route; installed CLI rejection route previously checked. Successful cloud loop open. |
| No enabled endpoint | Backend rejection and prior native dialog smoke passed locally. |
| Change enabled selection | Fixture proves resume keeps original binding after a changed stored selection. One-active-Pod policy prevents a normal concurrent two-Pod UI switch; full switch scenario not demonstrated. |
| Terminate enabled Pod | Local fixture confirms route invalidation and blocked resume. Actual deletion open; F1 affects local tools. |
| Streaming tool loop | Not tested with either model/CLI combination. |
| Two app sessions | Two fixture CLIs preserve folders/config directories and capacity. F5 affects row launch; real histories/model concurrency open. |
| App plus subscription CLI | Environment/config inspection and disposable fake normal-auth file only. Real side-by-side subscription and project override tests open. |
| Bad inference credential | Local HTTP and earlier real-binary localhost rejection probes. Real endpoint fallback/isolation gate open. F2 misclassifies transient errors. |
| Slow startup or no available GPU | Bounded code paths and mock lifecycle evidence; actual startup/stock failure open. F2/F3 affect cleanup. |
| Ambiguous create response | Mock lost response, late creation and duplicate handling pass. Real provider interruption open. |
| Interrupt during download/inference | Local basic PTY interrupt passes. F1 reproduces surviving descendants; actual runtime download/inference cancellation open. |
| Crash or laptop sleep | Durable reopen/revalidation tests. F3/F4/F6 remain; actual app crash/sleep with live Pod open. |
| Maximum lifetime while app offline | Limitation is visibly disclosed in source and prior native smoke. Independent expiry is neither implemented nor demonstrated. |
| Termination response lost | Local fixture stays pending until absence confirmed. Actual provider test open. |
| Retained cache selected | Payload/ownership preserve volume; client exposes no volume-delete operation. Real attachment/reuse/deletion preservation open; F7 remains. |
| Secrets in diagnostics | Explicit field allowlist, fake-secret file/argument checks and chunk redaction tests. No real-credential export exercised. |
| CLI/runtime upgrade | Exact CLI version/fingerprint/GPU/checklist gate tested locally. Updated combinations require new live evidence. |

## Checks and audit integrity

- `npm run check`: passed generated-contract drift check, Vue/TypeScript production build, Rust formatting and Clippy with warnings denied.
- `npm test`: passed 26 regular Rust tests and 5 mocked Python tests. The installed-CLI probe was ignored by the normal suite as designed.
- Earlier installed-CLI probe and native visual results are historical evidence from [local-validation.md](local-validation.md); they were not rerun or counted as new live evidence here.
- Eight additional diagnostic probes reproduced the seven findings: seven Rust observations plus the compiled Vue component observation. These assert the observed defects, not desired behavior; they are not additional “acceptance passes.”
- The temporary probes were removed from the normal suite after use. SHA-256 comparison confirmed all 44 inspected application/runtime/catalog/test files matched their pre-probe contents. Only audit/documentation changes are delivered by this audit.
- The existing app executable was present at `src-tauri/target/release/bundle/macos/Runpod Deck.app/Contents/MacOS/runpod-deck`, 11,729,856 bytes. SHA-256: `1052400e2f401a8029787979bf9b669cc619332f94513582abb21a96cff3b80e`. It was not rebuilt after this documentation-only audit and still contains these findings.
- No dependencies installed, normal CLI configuration changed, real credentials read, paid calls made, Git mutations performed, or project databases touched.

The original next step was to correct F1–F7, add regressions and rebuild; this is now recorded in the resolution table above. The owner can supply the remaining budget/lifetime settings and conduct the still-open live matrix later; passing local tests cannot replace those results.
