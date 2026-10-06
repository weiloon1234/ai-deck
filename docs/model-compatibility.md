# Model compatibility and owner-led cloud tests

Status: **No paid/live cloud tests performed.** Updated 6 October 2026.

The app implements the Codex and experimental Claude launch paths. It does not yet have a successful end-to-end self-hosted coding report for either CLI. Public Hub availability, a pinned image, localhost authentication tests, and compiled source are not substitutes for that report.

## Candidate catalog

Exact revisions, digest, parser flags, context limits, sampling values and hardware requirements live in [catalog.json](../model-catalog/catalog.json). Do not duplicate or change those values in deployment scripts.

| Profile | Proposed compute | Intended use | Live status |
| --- | --- | --- | --- |
| GPT OSS 20B | One H100 80GB | First Codex experiment; native MXFP4/Harmony path | Untested |
| Qwen3 Coder 30B | One H100 80GB | Second model on the shared vLLM runtime | Untested |

Both profiles propose a 16k context and two sessions. Neither capacity nor tool reliability has been measured. Both expose an experimental Claude option; this is not an Anthropic-supported configuration. The exact installed launcher versions are checked in the native adapters and displayed in Setup.

## First live experiment

Follow [Setup and provisioning](operating-guide.md). Set your total budget and maximum lifetime before enabling paid actions. Keep the app and Runpod console available throughout the test. Start with fully temporary storage and one session.

Use a disposable project containing a small function and a failing test. Ask the CLI to read both, fix the function, run the test with approval, then explain the output in a follow-up. Confirm the actual files changed locally. The app does not consider a text-only model response a coding-test pass.

Record the exact fingerprint shown by the app, profile/CLI versions, actual GPU type/count, region, date, startup/download duration, time to first response, estimated cost and final deletion evidence. Do not include keys or unredacted request headers.

## Live acceptance checklist

| Check | Evidence to record | Current status |
| --- | --- | --- |
| `streaming` | Incremental output arrives without broken completion events | Pending owner test |
| `file_read` | Correct contents of the disposable local file | Pending owner test |
| `file_edit` | Actual local edit and diff | Pending owner test |
| `safe_command` | Approval appears; a harmless test runs locally | Pending owner test |
| `tool_result_followup` | Follow-up uses the command output and correct tool-call identity | Pending owner test |
| `cancellation` | Interrupt stops generation/tool activity predictably | Pending owner test |
| `model_identity` | Correct served model and pinned launch configuration | Pending owner test |
| `context_limit` | Oversized input fails clearly without fallback or corruption | Pending owner test |
| `subscription_isolation` | Normal subscription CLI works alongside app session; original files unchanged | Pending owner test |
| `invalid_credential_no_fallback` | Bad inference credential fails; no subscription/model fallback | Pending owner test |
| `two_sessions` | Two distinct local projects and conversations; tested capacity | Pending owner test |
| `helper_models` (Claude only) | Helper requests use the selected gateway/model and complete correctly | Pending owner test |

Also complete the project plan's live lifecycle cases: public cold download, valid/invalid gated access, retained revision reuse, unavailable GPU, image/load/OOM failures, interruption during startup, reconnect/sleep/crash, lost API responses, and repeated verified termination. Mocked tests cover local controller behavior; they cannot establish the provider's actual behavior.

For every paid test, finish the deployment and confirm deletion in Runpod. Record any separately retained storage. Keep the independent-expiry gate pending until termination while the laptop is disconnected has actually been demonstrated.

## Recording evidence

Copy the test results into a new Markdown report under `docs/compatibility-reports/` after testing. Include observations, failures and limits; never turn an incomplete run into a passing report.

A profile's `evidence` item contains `cli` (`codex` or `claude`), the exact detected `cliVersion` string, actual `gpuType`, current `fingerprint`, `testedAt`, the report location, and its completed `checks` list. The required check names are authoritative in `model_catalog.rs`. The frontend does not manufacture evidence.

The fingerprint includes the immutable model configuration, runtime configuration and startup supervisor source. CLI version and GPU type must also match. A change invalidates old evidence; it never rewrites saved deployments. Imported catalogs must pass native semantic validation as well as the generated schema. Evidence is the owner's recorded attestation, not a cryptographic claim that a report was executed by the app.

If direct vLLM integration fails, record the precise Responses/Messages gap first. Add a protocol adapter only if that observed gap requires one. A bridge cannot supply missing model capability. Claude support remains experimental even after a successful report.
