# Implementation decisions and prerequisite audit

Recorded: 5 October 2026 (Asia/Kuala_Lumpur).

The [project plan](../runpod-coding-desktop-project-plan.md) remains the authoritative scope and acceptance checklist. The owner subsequently instructed implementation without paid tests and will perform cloud testing later. Local implementation and validation may proceed; live compatibility and release gates remain open. No paid resources have been created.

## Repository inspection

The destination initially contained only the project plan. It has no existing application, package manifests, or Git repository. No additional AGENTS.md was found in the destination or its parent directories. The instructions supplied by the owner in chat apply, including no commits, pushes, merges, dependency installation without approval, or destructive database operations.

The owner approved the proposed foundation and folder structure on 5 October 2026. Local implementation continued on 6 October. This decision record is the Phase 0 implementation decision record; the original plan is retained in place rather than duplicated.

## Installed tools

These are observed local versions, not a validated application compatibility matrix.

| Tool | Observed version |
| --- | --- |
| Host | macOS 26.6.2, build 25G83, arm64 |
| Node.js | 24.21.0 |
| npm | 12.0.2 |
| pnpm | 12.4.2 |
| Rust | 1.98.1 |
| Cargo | 1.98.1 |
| Installed Rust target | aarch64-apple-darwin |
| Xcode | 27.0, build 27A266a |
| Docker CLI / engine | 29.8.0 / 29.8.0 |
| Codex CLI | 0.160.0 |
| Claude Code | 2.1.289 |

The Docker engine responded to a read-only version check; no container was started and no image was downloaded. Project dependencies were installed after approval; exact resolutions are in package-lock.json and src-tauri/Cargo.lock. No toolchain upgrade was performed. The Codex version command succeeded with a sandbox warning about creating PATH aliases. No simplify skill was exposed in the available skill catalog or found by filename in the local skill directories searched; manual simplification reviews will be used.

## Approved owner decisions

On 5 October 2026, the owner replied “approve, please continue the goal.” This approves the proposed foundation, dependency installation, and initial lifecycle/scope choices below. It does not supply missing cloud spending limits or select a model.

| Decision | Approved choice |
| --- | --- |
| Desktop foundation | Tauri, Vue 3, TypeScript, Rust; section 10 organization |
| Interface and scope | Embedded terminals; Codex first; experimental Claude Code next; personal macOS build |
| Dependencies | Install the project dependencies required by the approved foundation |
| Storage | Ephemeral initially; retained network volume requires a separate selection |
| Route changes | Existing sessions retain their binding; new sessions use the newly enabled deployment |
| Window/session close | Keep the Pod running |
| App quit | Prompt to terminate active compute |
| Finish | Interrupt affected sessions, terminate the owned Pod, verify deletion |
| Optional phases 8–9 | Defer custom chat and Windows delivery unless requested |

## Cloud testing deferred by the owner

The owner delegated candidate model/GPU/region research, set a USD 10/hour GPU ceiling, and explicitly instructed “implement without paid tests first” and “i test later.” No paid provisioning or inference testing is authorized during implementation. Local mocked tests and build checks do not spend on Runpod.

The app starts with paid provisioning disabled. Before the owner provisions through the app, Setup requires a total experiment budget, maximum lifetime, and explicit acceptance of the offline cleanup limitation. No missing spending or lifetime values are invented. The hourly limit cannot exceed USD 10. Availability and pricing are rechecked using the owner's account when they choose to test.

The candidate catalog pins public model revisions and a common vLLM 0.31.0 image digest, obtained from Hugging Face metadata and Docker Registry manifest metadata on 5 October 2026. Exact values live only in model-catalog/catalog.json. Both candidates propose one NVIDIA H100 80GB HBM3 in a user-selected available Secure Cloud data center. No location, price, hardware capacity, model compatibility, or parser combination has been validated on Runpod. Every evidence list starts empty.

## Documentation checks

Reviewed current primary documentation on 5 October 2026:

- Tauri documents macOS Xcode and Rust prerequisites. The installed toolchain built the app; local verification is recorded in [local-validation.md](local-validation.md). [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/)
- Codex supports an app-owned state directory and custom providers. A launch must select the provider, Responses protocol, and explicit endpoint credential without changing normal subscription configuration. [Advanced configuration](https://learn.chatgpt.com/docs/config-file/config-advanced), [gateway configuration](https://learn.chatgpt.com/docs/enterprise/connect-to-a-gateway)
- Responses streaming and continuation, including tool calls and results, must pass real tests; a health check or text response is insufficient. [Gateway compatibility requirements](https://learn.chatgpt.com/docs/enterprise/gateway-compatibility)
- vLLM documents direct Codex and Claude Code integrations. Those examples are evidence for trying direct integration, not proof of compatibility or permission to select their example model, GPU count, context length, or unauthenticated settings. [Codex integration](https://docs.vllm.ai/en/latest/serving/integrations/codex/), [Claude Code integration](https://docs.vllm.ai/en/latest/serving/integrations/claude_code/)
- Anthropic states that routing Claude Code to non-Claude models is unsupported. A separate experimental validation gate remains necessary, with explicit gateway credentials as well as the base URL. [Gateway support and authentication](https://code.claude.com/docs/en/llm-gateway)
- Runpod's documented REST create request supports structured container entrypoint/arguments, environment, GPU selection, and network-volume attachment. The reviewed v1 create reference does not document a `terminateAfter` parameter. Do not invent a field or assume provider idempotency. [Create Pod reference](https://docs.runpod.io/api-reference/pods/POST/pods)

## Independent expiry: known provider limitation

Runpod removed `--stop-after` and `--terminate-after` in runpodctl v2.12.0 on 27 August 2026. Its merged removal report states that the GraphQL API accepted deadlines but left Pods running and billing past them. Scheduling values were write-only and unavailable in REST v2, preventing the client from verifying their application. Do not rely on older examples advertising these flags. [Removal report and provider test evidence](https://github.com/runpod/runpodctl/pull/330), [v2.12.0 release](https://github.com/runpod/runpodctl/releases/tag/v2.12.0)

On 5 October 2026, the restoration pull request remained open and explicitly depended on an API-side enforcement fix. Its proposed client validation does not prove that a timer fires. [Pending restoration](https://github.com/runpod/runpodctl/pull/331)

The latest release returned by GitHub was v2.14.0, published 10 September 2026. The inspected main revision was `4351fca9ec454b1bdc8572aaad5d3e5a61ead0fa`; its Pod create command and API request types have no deadline support. These are source checks, not live cloud tests. [Release](https://github.com/runpod/runpodctl/releases/tag/v2.14.0), [pinned create implementation](https://github.com/runpod/runpodctl/blob/4351fca9ec454b1bdc8572aaad5d3e5a61ead0fa/cmd/pod/create.go), [pinned GraphQL types](https://github.com/runpod/runpodctl/blob/4351fca9ec454b1bdc8572aaad5d3e5a61ead0fa/internal/api/graphql.go)

Runpod's management guide demonstrates a local delayed stop command. That is not independent termination, and stopped Pods can retain billable storage. [Pod lifecycle documentation](https://docs.runpod.io/pods/manage-pods)

Before claiming offline cost protection, verify the actual provider scheduling mechanism and demonstrate termination after the client disconnects. If it cannot be established, record that limitation and agree on an independent cleanup mechanism or explicit acceptance of the limitation before the paid experiment. A laptop timer must not be called a guaranteed spending cap. Never pass the Runpod management key to the model container.

## Selected candidates, not validated combinations

- [openai/gpt-oss-20b](https://huggingface.co/openai/gpt-oss-20b): the publisher documents MXFP4 weights, tool capabilities, and vLLM use. Its required Harmony format and actual runtime/GPU support need validation for the selected image.
- [Qwen/Qwen3-Coder-30B-A3B-Instruct](https://huggingface.co/Qwen/Qwen3-Coder-30B-A3B-Instruct): a coding-model candidate for further evaluation after budget and GPU constraints are known.

Both candidates now have immutable revisions and a pinned shared runtime in the catalog. GPU compatibility, available pricing and a real coding-loop result remain unverified. Neither is a tested catalog profile.

## Next gate

Deliver the locally built application under the owner's no-paid-tests instruction. Record local checks separately from live acceptance. The owner will perform the disposable Phase 1 coding experiment and subsequent cloud acceptance checks later. No phase with a live exit gate may be marked complete solely from code or mocked tests.
