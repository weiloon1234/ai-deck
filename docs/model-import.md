# Hugging Face model library

In **Models**, paste `https://huggingface.co/organization/model` and choose **Analyze & save**. A `/tree/revision` suffix is supported for a branch, tag or commit without slashes. The app resolves that reference to an immutable commit before reading the model config and card. It reads small metadata files only; importing does not download weights or create paid compute.

New analyses require Runpod selection and its management key in Setup, network access, and the selected local Codex CLI signed in with ChatGPT. GPUtw hardware analysis remains unavailable while its deployment integration is gated. Saved models and quotes have an explicit provider identity; older records default to Runpod. Refreshing a saved Runpod model uses Runpod even after the Setup selection changes. Any installed Codex version can be attempted; the app records its version without an exact-version allowlist. Required isolation flags remain in place, so an incompatible CLI may fail. Dated probe results are in [local validation](local-validation.md). Sign in using the CLI's normal login flow; do not paste subscription credentials into AI Deck. Analysis uses Codex subscription usage. Runpod coding sessions continue to use their separate isolated inference credentials.

## Saved results

Each entry contains:

- Original URL, pinned revision, model config/card metadata and access requirements, plus the Hub's reported parameter count and license when available.
- Codex's proposed GPU memory, GPU count, CPU memory, disk/cache capacity, reasoning and assumptions.
- A suggested usable context range and conservative default for **one coding session**, including model memory, KV cache and runtime overhead. These are estimates, not measured minima or throughput guarantees.
- Compatible Runpod GPU options with price snapshots, timestamps, and region stock signals. The native app calculates GPU count × provider unit price; Codex cannot supply a price field. Storage and other separately billed charges are excluded.
- An untested launch profile when the proposal fits the pinned runtime and the configured GPU budget (at most USD 10/hour).

The least expensive suggested GPU appears first. Stock signals do not guarantee that a particular GPU count is currently available. Without an explicit manual configuration, uncertain runtime/tool-calling proposals remain saved without a launch profile. **Edit configuration** lets the owner prepare an experimental trial. Full safetensors/PyTorch weights, a supported generation task, model config metadata, sufficient cache and a positive provider quote inside the hourly limit remain required. The app does not invent another model or quantization to fit the budget.

## Configure an experimental trial

There is no review queue or assigned reviewer. **Saved · Not configured** means no launch profile exists yet. Open **Edit configuration** on the saved model card, including models Codex marked uncertain.

1. Choose a Runpod GPU and count from current read-only hardware options. The editor shows total GPU rate, its own check time, and the hourly limit; storage is extra.
2. Adjust context, CPU memory, temporary disk and cache capacity for one coding session. Cache must exceed the reported full-weight size and leave working space. The model identity/revision and pinned runtime stay fixed in this editor.
3. Expand **Optional runtime settings** to edit tool-call parser, reasoning parser and quantization names. Empty fields omit their flags. Names must be built-in identifiers; the form cannot add commands, paths, arbitrary images, extra flags or remote-code execution. An unsupported combination may fail during startup or coding.
4. Choose **Try experimentally**. This saves your configuration separately from Codex's analysis and selects it in Deployment. It does not create a Pod. If another provider is selected, choose Runpod in Setup before preparing this Runpod model.

The resulting profile is labeled **Experimental · Untested** and has no passing evidence. Deployment still requires current availability, the hourly ceiling, total budget, maximum lifetime, paid-provisioning opt-in and explicit **Create paid Pod** confirmation. Ready/Enable still require the existing authenticated endpoint checks, and opening an untested CLI/model combination retains the candidate-test acknowledgement.

Failed or stale saves preserve the previous record and draft. Reopen the editor if the saved model changes while you are editing. Successful edits change the library's profile version/fingerprint; existing deployments and sessions keep their snapshots. Changing a profile also closes an old paid-creation confirmation, and Rust rejects requests for an outdated fingerprint.

## Model details in the app

Each imported model shows **Model details** beside **Suggested Runpod setup** in Models and when selected for deployment preparation. Facts include reported parameters, published config context limit, architecture, model type, precision, quantization, weight format/download size, task/library, license and access. Expand the source section for layers, attention/key-value heads, vocabulary size, expert counts, scaling settings and the saved config. Its Hugging Face link points to the pinned revision.

The published context value comes from the saved language config (including nested `text_config` for multimodal models). It is separate from Codex's proposed usable range and launch default. RoPE scaling is shown as metadata and never automatically multiplied into the displayed limit. A disabled sliding-window setting is labeled as disabled. Input and output share the context window.

Parameter counts use the Hub's `safetensors.total`; quantized tensor packing, adapters or mixture-of-experts models can make that different from original or active parameter counts. Unknown facts stay unknown; names and weight download sizes are not used to invent them. See [Hugging Face ModelInfo](https://huggingface.co/docs/huggingface_hub/package_reference/hf_api#huggingface_hub.ModelInfo) for the optional metadata source.

Existing saved entries immediately display facts from their stored config. Entries saved before parameter/license metadata was captured continue to load, with those fields shown as **Not provided**. **Analyze again** retrieves fresh metadata and also reruns Codex; reopening or selecting a model does not trigger another analysis.

The library uses the existing atomic local `state.json` store under `importedModels`; there is no new database dependency. Older files without this field load with an empty imported library. The bundled/file-imported catalog is retained, and imported profiles are appended when resolving the catalog. Existing deployments continue using their complete saved snapshots.

## Reuse and refresh

- Pasting an already saved URL reuses its analysis immediately, including while offline. The plain repository URL and `/tree/main` identify the same entry.
- **Refresh prices** reads the saved provider again without consuming another Codex analysis. It uses your manual GPU configuration when present, otherwise the AI's suggested hardware. If no quote fits the limit, the manual settings remain saved while the launch profile becomes unavailable.
- **Analyze again** explicitly fetches the URL's current revision and replaces the saved AI analysis only after a complete valid result. Your manual experimental settings are retained. A failed or cancelled run preserves the previous entry.
- **Use saved suggestion**, or **Try experimentally** for a saved manual profile, selects its deployment profile. Creating a Pod still requires the existing paid-provisioning opt-in, budget/lifetime, current-price checks and explicit creation confirmation.
- Removing a library entry preserves existing deployment/session snapshots. It does not remove projects, credentials, Pods or retained volumes.

Imports have their own lock and cancellation path. Analysis never holds the deployment operations lock, so Finish and reconciliation remain available. Quit cancels and waits for analysis cleanup. Codex has a four-minute analysis timeout; metadata requests are bounded and cancellable.

## Integration boundaries

The analyzer checks `codex login status` for ChatGPT login, retains the normal authentication home and excludes inherited API keys and endpoint overrides. It uses `exec --ignore-user-config --ignore-rules --ephemeral --sandbox read-only`, a private temporary working directory, and a generated strict JSON output schema. It does not copy credentials or write normal config. It deliberately avoids `forced_login_method`, which can log out mismatched credentials. The CLI may perform its own ordinary authentication refresh.

The analyzer config disables project instructions, apps, plugins, hooks, browser/computer access, shell execution, subagents and the code-mode executor. The previously checked Codex builds still advertise generic execution and question wrappers; the execution host is disabled and the CLI reports that it fails closed. The optional installed-CLI check verifies the restricted declarations and structured response format against a rejecting localhost server. Other versions remain unverified, and managed administrator requirements can affect CLI behavior. See the [CLI reference](https://learn.chatgpt.com/docs/cli/reference) for the documented execution flags; the installed-binary probes also check version-specific behavior.

Model-card text is treated as untrusted input. Native validation rejects unknown analysis fields, invalid GPU IDs, unreasonable values and unsafe parser/quantization strings. AI output never controls provider URLs, credentials, container images, shell commands or remote-code execution. Publicly readable metadata is required for URL import; private repositories can still use a manually prepared catalog. Gated model downloads later require the existing Runpod Secret setup.

Sources used for the integration: [Codex non-interactive execution](https://learn.chatgpt.com/docs/non-interactive-mode), [Codex configuration](https://learn.chatgpt.com/docs/config-file/config-reference), [Codex authentication](https://learn.chatgpt.com/docs/auth), [Hugging Face model metadata](https://huggingface.co/docs/huggingface_hub/package_reference/hf_api#huggingface_hub.HfApi.model_info), [Runpod GPU queries](https://docs.runpod.io/sdks/graphql/manage-pods), and [vLLM tool calling](https://docs.vllm.ai/en/v0.31.0/features/tool_calling/). Exact CLI flags were also checked against the installed binary.

## Validation

Local regression tests exercise URL restrictions, pinned metadata, malformed analysis, subscription-vs-API login selection, timeout cleanup, persistent cache/restart, quote calculation, budget handling, unsupported models, safe reanalysis, cancellation during cloud cleanup, and the actual Vue event handlers. A separate read-only Hugging Face check fetched public model metadata/config/card successfully. The installed Codex probe used a disposable login home and a localhost rejection server.

No successful real-subscription analysis or paid Runpod inference test was run during implementation. The first real analysis, account-specific Runpod price query and GPU run remain owner-operated checks. Imported suggestions must not be represented as live-validated model compatibility.
