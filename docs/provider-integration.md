# Cloud provider integration

Assessment: **10 October 2026**, setup integration updated **13 October 2026**. Runpod and GPUtw are now selectable in Setup; Runpod remains the only paid deployment adapter. GPUtw has much of the required API, but its documented contract does not yet establish everything needed for AI Deck's ownership and uncertain-creation recovery. No real GPUtw key, account state, instance, model download or paid inference was used. The separately authorized local wrapper build downloaded the pinned public container layers; its verification and publication status are recorded below.

## Provider setup implemented — 13 October 2026

Setup now has an immediately saved provider selector, separate API-key fields/Keychain entries, read-access checks and clear GPUtw availability guidance. The workspace header and Deployment screen identify the selected provider. GPUtw's deployment gate replaces Runpod creation controls and is also enforced natively before any paid request or saved creation intent. Spending opt-in cannot bypass it.

`ProviderId` defaults older settings, deployment records, imported analyses and quotes to Runpod. `CloudProviders` routes saved deployment ownership, inspection, reconciliation, Enable and Finish by that recorded identity. Changing the selected provider does not retarget existing sessions, alter their runtime/profile fingerprint or replace their management key. Keys cannot be changed while their own provider has active deployments; the other key can still be configured. Runtime routing explicitly refuses unsupported provider identities instead of constructing a Runpod URL.

`provider_service.rs` owns selection and key operations. `gputw_client.rs` implements only the documented read-only `GET https://gputw.ai/api/instances` check, using `instances:read`, bounded responses/timeouts and no redirects/proxies. It discards upstream records, sanitizes failures and reports read access only. This does not test other scopes, account billing, ownership marker readback, startup or recovery. [GPUtw authentication example and scopes](https://docs.gputw.ai/zh-TW/docs/api-keys).

Provider metadata for the browser preview is generated from Rust alongside contracts/defaults. Cached hardware quotes, location/storage selection and the creation confirmation are cleared on provider change; late responses from a previous provider are discarded. Existing Runpod model quotes stay labeled and bound to Runpod. New analysis requires Runpod selection; saved-price refresh uses the record's original provider.

This completes visible setup and provider binding for the current Runpod lifecycle, not the GPUtw paid adapter. GPUtw still needs its own concrete-node quote/storage selection, runtime-image binding, endpoint/port setup, creation/ownership recovery and verified deletion implementation after the contracts below are confirmed. No supported GPU combination or live test is inferred from fixtures. [Current local validation](local-validation.md#provider-setup-and-routing--13-october-2026).

## GPUtw fit

### Contract recheck — 13 October 2026

Rechecked the current official pages and anonymously fetched the vendor reference repository at commit `136fa0bf6bcd9870e2b860372db89931005d93ea`. Read its actual create schema, HTTP client, response projection and storage guides without installing or executing vendor code. The public issue list was empty at the check; no existing vendor answer established the missing contracts. No real GPUtw key/account, instance creation, image validation, storage write or paid inference was used.

| Contract | Confirmed from primary sources | Still needs vendor confirmation |
| --- | --- | --- |
| Creation/recovery | Custom-image `env` is accepted and saved with the instance. List/get use the same instance format. Create is a write and is not described as idempotent. [Environment persistence](https://docs.gputw.ai/zh-TW/docs/monitoring), [list/get](https://docs.gputw.ai/zh-TW/docs/instance-runtime), [pinned create schema](https://github.com/GPUtw-ai/GPUtw-Skill/blob/136fa0bf6bcd9870e2b860372db89931005d93ea/mcp/src/tools/instances.ts) | The exact stable env-marker response fields, availability before startup/after failure, complete enumeration, or a supported create idempotency mechanism. The vendor MCP projection omits fields; that is not proof the underlying API lacks them. |
| Startup | A custom image needs SSH or a responding declared Web UI port, with an approximate 15-minute failure timeout. [Templates](https://docs.gputw.ai/zh-TW/docs/templates) | TCP versus HTTP probe, method/path/accepted codes/authentication, and the timeout's starting point. An HTTP 401 response is not evidence that the platform accepts it as ready. A separate minimal liveness endpoint is a possible implementation after confirming the probe; protected status/inference would retain authentication. |
| Workspace | The rented node determines storage; `df` reports host space and must not be treated as enforced quota. `/instances/{id}/resources` reports allocation/usage, with `null` meaning unknown. [Docker environment](https://docs.gputw.ai/zh-TW/docs/docker-environment), [resources](https://docs.gputw.ai/zh-TW/docs/instance-runtime) | Exact workspace-used fields/units/sample validity, decimal GB versus GiB, what counts toward the limit and whether create can select capacity. The example resources schema gives `allocated.diskSizeGb` but does not specify a disk-usage metric. |
| Vault | Shared `/vault` persists beyond instance deletion. The reference documents `/vault/stats` fields `usedGb`, `quotaGb`, `vaultSizeGb`, `vaultStorageRateHr`. [Vault](https://docs.gputw.ai/zh-TW/docs/vault), [pinned storage reference](https://github.com/GPUtw-ai/GPUtw-Skill/blob/136fa0bf6bcd9870e2b860372db89931005d93ea/guides/06-vault-basics.md) | Exact field types/units/rate currency and accounting for concurrent downloads/reservations. Account quota must be checked separately from workspace capacity. |

An anonymous read of the documented `/config/deploy` endpoint confirmed the included 100 Mbps tier, the paid 400 Mbps tier and `internetPricePerMbpsHr: 0.003` on this date. These are public configuration observations, not a reservation or owner-specific quote. No paid tier was selected.

The native read-access client omitted the `User-Agent` required by the vendor's front door. It now sends the app name/version plus `Accept: application/json`; loopback fixtures assert those headers. [Pinned vendor client](https://github.com/GPUtw-ai/GPUtw-Skill/blob/136fa0bf6bcd9870e2b860372db89931005d93ea/mcp/src/client.ts). This corrects transport behavior; it does not establish creation or runtime contracts.

After the owner's explicit approval, the prepared clarification was posted on **13 October 2026** as [GPUtw-Skill issue #1](https://github.com/GPUtw-ai/GPUtw-Skill/issues/1). It requests redacted JSON examples, pre-start/FAILED ownership-marker guarantees, readiness-probe details and workspace/Vault units. The GitHub connector was denied repository write access, so the approved text was posted through the owner's signed-in browser and the created issue was verified. No vendor reply has been verified yet. A vendor reply can resolve these contracts without renting a GPU. Full implementation can then use local protocol/lifecycle/runtime fixtures; successful GPU loading, coding and real deletion remain later owner-operated acceptance. The paid adapter is not enabled by this recheck, and no unsupported marker field/header or guessed quota is added to production.

| Requirement | Documented support and integration impact |
| --- | --- |
| Discovery | Public GPU catalog and scoped available-node lookup. Selection is by a concrete node, with architecture, RAM, storage and whole-machine hourly rate; this differs from Runpod's GPU/data-center selection. [Catalog API](https://docs.gputw.ai/zh-TW/docs/gpu-catalog) |
| Custom runtime | Custom images accept environment variables and arguments, but arguments replace `CMD` while preserving `ENTRYPOINT`. AI Deck currently overrides the upstream vLLM entrypoint to run its Python supervisor. GPUtw therefore needs a published, pinned wrapper image. [Create API](https://docs.gputw.ai/zh-TW/docs/rest-api-quickstart) |
| Inspect and terminate | List, get, status, stop and delete operations are documented. Account ownership alone does not distinguish resources created by AI Deck from the owner's other workloads. [Instance API](https://docs.gputw.ai/zh-TW/docs/instances), [runtime API](https://docs.gputw.ai/zh-TW/docs/instance-runtime) |
| Inference access | HTTP endpoints use `<port>-<instance-id>.gputw.ai`. Private and unlisted modes require browser authentication; coding CLIs need public routing with the supervisor/vLLM bearer protection retained. Port configuration requires `ports:manage` after the instance is running. [Ports](https://docs.gputw.ai/zh-TW/docs/ports) |
| Storage | `/workspace` follows the instance; `/vault` persists and is shared across the account's instances. Vault needs its own capacity and cost checks, rather than being treated as a Runpod network-volume ID. [Vault](https://docs.gputw.ai/zh-TW/docs/vault) |
| Credentials | Expected management scopes are `catalog:read`, `instances:read`, `instances:create`, `instances:manage` and `ports:manage`. Root-level `instances:exec` is separate and should not be required simply to establish resource ownership. [API keys](https://docs.gputw.ai/zh-TW/docs/api-keys) |
| Offline cleanup | The per-instance self-stop token can request termination without an account management key. It is a possible future cleanup mechanism; a documented request is not evidence of a verified deadline or deletion after the Mac disconnects. [Self-stop](https://docs.gputw.ai/zh-TW/docs/instance-runtime) |

## Price and capacity observations

The vendor's reference identifies API rates as **USD/hour**, while prepaid credits are displayed in TWD. Extra bandwidth, ports and retained Vault storage can add charges. [Billing reference](https://github.com/GPUtw-ai/GPUtw-Skill/blob/master/guides/09-account-billing-teams.md)

The read-only public catalog returned these examples on the assessment date:

| GPU | Catalog `hourlyPrice` | Available offer `hourlyRate` |
| --- | --- | --- |
| RTX 3090 24GB | $0.12 | $0.3031 |
| RTX 5090 32GB | $0.32 | $0.5994 |
| RTX PRO 6000 WS 96GB | $0.83 | $1.6250 |

These are dated public observations, not account-specific reservations or a like-for-like Runpod comparison. Provisioning must use the selected node's total quote and then check the created instance's actual rate, rather than multiplying the advertised catalog rate. Refresh availability before creation. [Public catalog](https://api.gputw.ai/api/gpus/active), [quote semantics](https://docs.gputw.ai/zh-TW/docs/gpu-catalog)

No rentable H100 offer was returned. The bundled profiles declare H100 hardware; they must not be silently remapped to an RTX GPU or an arm64 Spark. New combinations require explicit profiles and their own compatibility evidence.

For large models, the available-node `rentableStorageGb` and the returned instance `diskSizeGb` must cover the profile's cache requirement. The inspected create schema does not document a configurable disk-size field. Vault quota also needs confirmation before using it for a 500GB cache. The vendor reference lists included 100Mbps bandwidth; at that speed, transferring 500 decimal GB takes about 11.1 hours even before overhead. That calculation is not a measured download rate and exceeds the current catalog download timeout. [Capacity reference](https://github.com/GPUtw-ai/GPUtw-Skill/blob/master/guides/02-catalog-and-capacity.md), [deployment fields](https://github.com/GPUtw-ai/GPUtw-Skill/blob/master/guides/03-instances-lifecycle.md)

## Contracts needed before enabling GPUtw

1. **Recoverable creation identity.** Confirm a supported client request ID, tag or custom environment field that survives create/list/get and can identify this installation and deployment after a lost create response. The inspected request/response reference and vendor MCP create schema do not document one. This is an unresolved contract, not proof that the service lacks the feature. Never substitute account access, creation time or matching hardware for app-specific ownership. [Lifecycle reference](https://github.com/GPUtw-ai/GPUtw-Skill/blob/master/guides/03-instances-lifecycle.md), [vendor create schema](https://github.com/GPUtw-ai/GPUtw-Skill/blob/master/mcp/src/tools/instances.ts)
2. **Startup contract for the published supervisor image.** The wrapper is now public with a verified immutable digest recorded below. Confirm that GPUtw's required SSH/Web UI readiness permits an authenticated status service during long weight downloads. The vendor reference requires one of those interfaces. Do not bypass supervisor authentication or enable root SSH just to satisfy a health check. [Custom-image requirements](https://github.com/GPUtw-ai/GPUtw-Skill/blob/master/guides/03-instances-lifecycle.md)
3. **Provider-bound records and selections.** The 13 October setup work adds the persisted identity, old-record Runpod default, credential separation and saved-provider routing described above. GPUtw's concrete node quotes, enforced storage, wrapper image/runtime fingerprints, endpoints/ports and paid lifecycle still need their adapter-specific implementation. Changing a selection must never retarget existing sessions.

Owner-led live acceptance must still prove authentication rejection, startup, coding compatibility, uncertain-response recovery and verified deletion. A public metadata fetch does not satisfy those gates.

## Preparing the wrapper image locally

[`containers/vllm-runtime/Dockerfile`](../containers/vllm-runtime/Dockerfile) wraps the existing supervisor, clears the upstream command and retains its authenticated status/inference behavior. It installs no packages and uses a minimal build context. The base digest remains in the model catalog; the recipe contains no second image/version constant.

Generate a plan using the selected runtime ID from `model-catalog/catalog.json` and a registry/repository you control:

```sh
python3 scripts/prepare_runtime_image.py \
  --runtime-id vllm-0-31-0 \
  --image-repository registry.example.com/your-team/ai-deck-runtime
```

This only prints JSON: the pinned base image, source hashes, `linux/amd64` platform, a derived local image tag and structured `buildArgv`. It never executes those arguments or contacts Docker/a registry. The tag changes when the base, supervisor, Dockerfile or context filter changes; it is a recipe identity, not the resulting registry digest or the app's profile fingerprint. Architecture is explicit: this recipe does not establish arm64/Spark compatibility.

After publication, record the actual immutable wrapper digest in the eventual provider-bound runtime configuration. Do not use the local tag as a deployment pin or replace the Runpod base image with a wrapper tag. Vendor readiness handling and provider-enforced storage capacity still need validation: container filesystem free space alone cannot establish GPUtw's quota. [Quota semantics](https://docs.gputw.ai/zh-TW/docs/docker-environment)

### Selected image and local verification — 10 October 2026

The owner selected the public package `ghcr.io/weiloon1234/ai-deck-runtime`, with source repository `https://github.com/weiloon1234/ai-deck-runtime`. The plan included `--source-repository` so the OCI repository label participates in the recipe identity.

| Item | Observation |
| --- | --- |
| Local image tag | `ghcr.io/weiloon1234/ai-deck-runtime:ai-deck-811ed61312a2eddccefe0bfa` |
| Platform | `linux/amd64`; built on the Mac using Docker's platform support |
| Recipe fingerprint | `811ed61312a2eddccefe0bfa7b4bca552c5a77f47312cc29fb1961947375ee97` |
| Local image ID | `sha256:7db2ab1763ff2a01a77ebbc08eef2647fb49ef1000840ff02f509fccecf42447`; this is local build evidence, not a verified published deployment pin |
| Build | Passed against the catalog's pinned upstream vLLM base, with only the recipe and supervisor in the context. No packages or model weights were added. Docker reports 9,035,268,423 image bytes. |
| Startup check | Passed with no network, GPU or host project mounts, a read-only filesystem, fake credentials and resource limits. The actual image entrypoint served authenticated `/status`; missing/wrong tokens were rejected, unknown authenticated paths returned 404, and the launcher hash matched the plan. No inference readiness was claimed. |
| Cleanup | The one disposable fixture container was removed and its absence verified. |
| Publication | Not published at the 10 October check. The subsequent authenticated upload attempts and prepared workflow are recorded below. |

The build's `InvalidDefaultArgInFrom` advisory reflects the deliberately required `BASE_IMAGE` argument. The plan supplied the validated catalog digest; no fallback image was duplicated in the recipe. GPU initialization, model loading and GPUtw's external health check were not tested.

### Verified registry upload — 13 October 2026

The owner resumed publication. The current supervisor, Dockerfile, context filter and catalog base still match the verified recipe. All 42 archive blobs were hashed before authentication. GHCR reused 41 existing blobs and accepted the remaining 5,153,216,601-byte layer. Authenticated readback verified the exact child manifests, index and derived tag.

Registry image: `ghcr.io/weiloon1234/ai-deck-runtime@sha256:fda0b0519a13eb0ccb9b76faa40908dc37c0a4a3eb87f82c646dc37a71bbde07`. This is now a verified registry identity rather than a local-only ID. Its Linux/amd64 runtime manifest and config match the earlier isolated CPU verification. No GPU, model weights or paid inference were used.

The successful direct HTTPS layer stream took 815 seconds with a bounded 30-minute client wait. Earlier short-cutoff attempts were stopped before confirmation, including a briefly duplicated restart that was corrected. The observed success supersedes the earlier inferred transfer blocker; it does not establish a new vendor timeout guarantee. No Git commit, push, merge, GitHub workflow run or dependency change was performed.

Public visibility is confirmed after the owner's fresh approval, and the package is connected to the selected `weiloon1234/ai-deck-runtime` repository. [Public package](https://github.com/weiloon1234/ai-deck-runtime/pkgs/container/ai-deck-runtime). Anonymous GHCR requests verified the exact index digest, its single runnable Linux/amd64 manifest, the matching runtime config reference and the largest layer's digest/size through HEAD. The verifier used no owner credential helper or management key and downloaded no image layers. The initial automatic approval rejection was resolved; no package setting remains pending.

Use the immutable registry image above when a GPUtw adapter is eventually configured. Keep the current Runpod base and saved deployment snapshots intact. The availability gate is complete; GPUtw creation recovery, startup/readiness, storage quotas, provider-bound persistence and real owner-operated GPU acceptance remain separate gates. No GitHub repository content or workflow was published or executed; linking the package does not commit source files.

### Publication attempts and prepared GitHub workflow — 11 October 2026

The owner's GHCR Docker login works and the GitHub browser is signed in. Publication is still incomplete: authenticated registry HEAD checks confirmed **41 of 42 blobs**, with the **5,153,216,601-byte** layer still missing. No final registry digest, public package visibility or anonymous pull verification is claimed.

The owner confirmed the Mac is using mobile data and requested a pause until returning to high-speed home Wi-Fi. All publication clients have stopped. Preserve the verified local tag/archive and working GHCR login; retry only after the owner resumes, then confirm the actual registry digest, Public visibility and anonymous access. The prepared GitHub workflow below is an optional alternative, not a required Git change for the Wi-Fi retry.

The initial Docker upload failed without an established cause. A retry was stopped after 13 minutes 56 seconds; an existing-builder export then reached its own 30-minute deadline. Those attempts uploaded reusable content but did not confirm an image manifest. A direct Mac HTTPS stream sent about 750 MB in nine minutes before its own 570-second deadline. The observed rate was about 1.3–1.6 MB/s; sending this layer within GHCR's documented 10-minute upload limit requires at least 8.6 MB/s before overhead. The layer is below GHCR's 10 GB size limit. [GHCR limits](https://docs.github.com/en/packages/working-with-a-github-packages-registry/working-with-the-container-registry#troubleshooting)

The ranged-chunk fallback passed eight offline scope/recovery fixtures, but GHCR rejected the first ranged PATCH with 416 and its status GET returned a 303 outside the registry repository. A separate two-chunk check using an existing two-byte config blob also failed final digest confirmation (`BLOB_UPLOAD_INVALID`). Credentials were never forwarded to the redirect destination. These observations do not establish working resumable uploads on this service.

The cached builder refreshed the local index to `sha256:fda0b0519a13eb0ccb9b76faa40908dc37c0a4a3eb87f82c646dc37a71bbde07`. The runtime manifest (`sha256:4a6413d73df01cf5f74591b20f79d393a474f71ba76df9fbf79e5513ed4352d3`) and config (`sha256:580296023f13ebb8e68859070d2435c04e33b0c95719b0b0752a7264cde7430a`) remain unchanged. The updated local image passed the isolated supervisor/authentication fixture again, and all 42 payload hashes in its 9,035,314,176-byte OCI archive were verified. These remain local identities, not published pins.

A small source bundle is prepared for the owner's `weiloon1234/ai-deck-runtime` repository. It includes the unchanged Docker recipe/supervisor/context filter, a catalog-derived plan snapshot, a manual-only GitHub Actions workflow, CPU verification and publishing helpers, and image-specific guidance. Five offline fixtures cover source/target rejection, runner guards, failed-verification publication blocking and credential handling; source hashes, Python/shell syntax and ZIP integrity also passed. **The GitHub workflow has not been executed.**

The owner must copy/commit the bundle to that repository because this project's rules prohibit agent Git commits and pushes. Its standard public `ubuntu-24.04` runner uses the temporary `GITHUB_TOKEN` with `contents: read` and `packages: write`; no extra PAT secret is needed. It frees unused preinstalled tools only on the disposable hosted runner, requires 48 GiB free, builds the pinned base, verifies the actual supervisor without network/GPU, then uploads the checked image and reports the real registry digest. It never selects a larger paid runner or creates GPU resources. Set the resulting package to Public and verify anonymous manifest access afterward. [Workflow authentication](https://docs.github.com/en/packages/working-with-a-github-packages-registry/working-with-the-container-registry#authenticating-in-a-github-actions-workflow), [standard public runners](https://docs.github.com/en/actions/reference/runners/github-hosted-runners)

### Publishing the selected package

Authenticate locally with a GitHub token (classic) scoped to `write:packages`, using Docker's hidden password prompt. Do not paste the token into chat or save it in project files:

```sh
docker login ghcr.io -u weiloon1234
```

Once login is available, upload the verified local tag. Set the package's visibility to **Public** in its GitHub package settings, then verify its manifest anonymously and record the registry-returned digest. GitHub initially creates private packages even when the linked source repository is public. The OCI source label declares the repository; if the association is not visible, use Connect Repository for the selected repository, as this publication did. Neither labeling nor connecting uploads source files. No Git commit or Git push is part of this work. [GHCR publishing and repository linking](https://docs.github.com/en/packages/working-with-a-github-packages-registry/working-with-the-container-registry)

## Checking marker readback without provisioning

Further source review found that GPUtw documents saving custom environment values with the instance for later reuse. The vendor's MCP tool deliberately projects a smaller set of response fields, so absence from that tool's output does not prove absence from the underlying REST API. The exact stable readback fields still need confirmation. Environment values are documented as stored unencrypted; account management keys must never enter container environment variables. [Environment persistence](https://docs.gputw.ai/zh-TW/docs/monitoring), [response projection source](https://github.com/GPUtw-ai/GPUtw-Skill/blob/master/mcp/src/project.ts)

[`scripts/probe_gputw_recovery.py`](../scripts/probe_gputw_recovery.py) checks an **existing owner-selected instance** using only `GET /instances` and `GET /instances/{id}`. It does not create a test instance, set environment variables, change ports, download files or call inference. It does not read AI Deck's state or Keychain. The API key needs only `instances:read` and is entered without echo in an interactive terminal; never paste it into chat, a command argument or a repository file.

Use a record that already contains two known, non-secret UUID environment markers for ownership and deployment identity. If there is no such existing record, this probe cannot settle the question without a separately authorized experiment or vendor confirmation. Do not create or reconfigure a workload merely to run this probe under the current no-paid-tests instruction.

From the project root, replace the uppercase UUID placeholders before running:

```sh
python3 scripts/probe_gputw_recovery.py \
  --instance-id INSTANCE_UUID \
  --marker RUNPOD_DECK_OWNER=OWNER_UUID \
  --marker RUNPOD_DECK_DEPLOYMENT=DEPLOYMENT_UUID
```

The marker names above correspond to the existing Runpod ownership protocol; the probe accepts explicitly supplied names so it does not define another production protocol. Marker values are non-secret identifiers, not inference keys or credentials. The probe inspects object maps and `name`/`value` arrays without printing upstream records or other environment values. It uses a fixed HTTPS host, disables ambient proxies and redirects, limits each response to 2 MiB and sets a 20-second socket timeout.

Exit `0` means the selected instance appeared once and its expected marker pair was consistently observed in both responses. Exit `2` means readback remains unconfirmed; exit `1` means a sanitized transport/protocol/input failure. Missing markers, conflicting values, multiple matching instances or mismatched resource IDs never become a passing observation. The result always reports `idempotency: not_verified` and `production_recovery: not_enabled`. This checks readback for that record, not a contractual persistence guarantee or recovery before a container starts.

If readback is confirmed and the stable field/schema is documented, the GPUtw adapter can send the existing UUID markers, normalize them from list/get, and reuse the current no-retry reconciliation and ownership gates. If the API exposes no such field, GPUtw needs to provide creation correlation or idempotency; adding an unsupported header in AI Deck cannot create that server behavior. Authenticated runtime proof would be a separate design and would not cover containers that never start. Automatic recovery remains disabled until a sufficient contract is established.

A precise vendor question is: “Does `POST /instances/create` honor an idempotency key, including retries after a lost response? If not, which stable fields in `GET /instances` and `GET /instances/{id}` return custom environment ownership/deployment markers, including while DEPLOYING or FAILED?” This text was prepared without sending it during the initial assessment. The expanded questions were subsequently posted, with owner approval, as [issue #1](https://github.com/GPUtw-ai/GPUtw-Skill/issues/1) on 13 October 2026.

## Confirming contracts without renting a GPU

Vendor documentation or a written support answer can establish the API contract before any paid test. An existing appropriately marked instance can supply a read-only observation using the probe above. Neither method replaces eventual real startup, lost-response recovery and verified-cleanup acceptance.

The remaining questions for GPUtw are:

1. **Creation recovery:** does create support a documented idempotency key (name, retention period and retry behavior)? Otherwise, exactly which list/get fields return both custom environment UUID markers before the container starts and after a failed deployment? Are records paginated, and how do we enumerate every possible match after a lost response?
2. **Startup:** when `sshEnabled` is false and `webUiPort` is the supervisor's status port, is readiness a TCP check or an HTTP check? For HTTP, specify the method/path, accepted status codes, authentication support and timeout. The supervisor starts before model download, rejects unauthenticated requests with 401 and serves authenticated `/status`; inference remains unavailable until model loading completes. Is that sufficient to become RUNNING during a long download without enabling SSH?
3. **Storage and billing:** which fields/units in the available-node, instance and resources responses describe enforced workspace allocation and current usage, and which API reports Vault quota/usage? Confirm when custom-image billing begins. The basic instance page says RUNNING, while the custom-image page includes image-pull time; do not assume an unsuccessful deployment is a free experiment. [Instance lifecycle](https://docs.gputw.ai/zh-TW/docs/instances), [custom-image requirements](https://docs.gputw.ai/zh-TW/docs/templates), [workspace limits](https://docs.gputw.ai/zh-TW/docs/docker-environment)

The wrapper image can be built locally and published separately from GPUtw. This requires downloading container layers, a working local Docker engine, the owner's chosen registry/repository/visibility and authenticated publishing access; no GPU instance or model weights are needed. GHCR uses the owner's existing GitHub account, supports repository-linked packages and currently has free container-image storage/bandwidth. Publishing still needs authentication, and new packages default to private until their visibility is changed. [GHCR authentication and linking](https://docs.github.com/en/packages/working-with-a-github-packages-registry/working-with-the-container-registry), [current billing](https://docs.github.com/en/billing/concepts/product-billing/github-packages)

The owner selected `ghcr.io/weiloon1234/ai-deck-runtime`, linked to `https://github.com/weiloon1234/ai-deck-runtime`, with public visibility. Add `--source-repository https://github.com/weiloon1234/ai-deck-runtime` to the preparation command. This binds the OCI repository label into the recipe identity. A published registry digest and anonymous manifest readback must be recorded before treating the image as available to GPUtw; a local image tag is insufficient.

## Initial boundary refactor — 10 October 2026

[`CloudProvider`](../src-tauri/src/cloud_provider.rs) replaces the Runpod-named service boundary and holds the shared resource/hardware types. Providers build their own opaque creation requests and perform their own ownership checks. Provisioning, reconciliation, Enable and Finish call those hooks; durable creation intent and deletion verification remain in the application services.

[`RunpodClient`](../src-tauri/src/runpod_client.rs) now owns the existing Runpod payload builder and ownership protocol. Shared bounded HTTP handling lives in [`http_client.rs`](../src-tauri/src/http_client.rs), so runtime and Hugging Face clients no longer import HTTP utilities from the Runpod adapter. Inference credentials are saved only after request preparation succeeds.

At that date this was an initial boundary refactor, not an enabled multi-provider registry. The 13 October setup work adds the registry and saved identities; the GPUtw paid adapter remains gated. Existing serialized contracts and Runpod ownership keys retain their names. The vendor reference repository was read as API documentation; no external skill or code was installed or executed. Native regressions and generated contracts now pass after owner-approved locked dependency restoration; current packaging results and live limits are recorded in [local-validation.md](local-validation.md).
