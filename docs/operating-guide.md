# Operating AI Deck

To add more models, open **Models**, paste a Hugging Face model URL, and choose **Analyze & save**. The app uses your signed-in local Codex CLI to suggest hardware and a usable context range for one coding session, reads Runpod GPU pricing, and saves the result locally. **Refresh prices** reuses the saved analysis; **Analyze again** updates the model revision and suggestion. See [model import](model-import.md) for setup and limitations. Importing does not provision paid compute.

## Before your first cloud test

No cloud resources or paid inference were used to develop this build. You control the first paid experiment.

1. Open **Setup**. Add your Runpod management key; it is stored in this app's Mac Keychain entry. The frontend clears the input after submission.
2. Confirm the detected Codex / Claude paths. The app checks exact versions and refuses unknown launcher versions rather than weakening isolation. It does not install or update either CLI.
3. Set a total experiment budget and a maximum Pod lifetime. Leave the hourly limit at or below USD 10. Saving smaller limits can shorten an active Pod's deadline; larger values never extend that deadline. The total is cumulative across the app's saved deployment history; increase it deliberately for additional experiments. Storage and provider billing differences are not included in a guaranteed cap.
4. Read the offline cleanup limitation. When you are ready, acknowledge it and enable paid provisioning. This setting alone does not create compute.
5. Add a **disposable local project** in Projects. Keep real databases and important working files out of initial coding tests.

The recommended first candidate is GPT OSS 20B on one H100 80GB. Qwen3 Coder 30B uses the same image as the second candidate. Both require live compatibility validation. Start with temporary storage. Select a location from the current availability list; a nearby location such as Singapore is preferable when offered at a rate within your limit. No region or price is assumed to be available.

## Provision → Enable → Open CLI

1. Select a model in Models or Deployment. Review its limitations and version details.
2. **Check availability**, select a currently available location, and optionally select an existing network volume in that location. The app never creates or deletes network volumes.
3. **Provision Runpod** opens a cost confirmation. Only **Create paid Pod** submits a creation request. Pricing is rechecked before creation and again after the provider reports the actual Pod rate. Over-limit or missing actual prices trigger cleanup, but short startup charges can still occur.
4. Watch Downloading, Loading, and Verifying. The status service requires the deployment credential. Ready requires rejecting a bad credential, matching the served model name, and returning an inference result. Ready is not proof of reliable coding ability.
5. Select **Enable endpoint**. This verifies it again and selects it for new local sessions. Enable does not create another Pod or start billing; billing already began during provisioning.
6. **Open CLI**, choose a project and CLI, name the session, and acknowledge candidate testing until exact compatibility evidence exists. Normal CLI approvals remain active.
7. Use the session sidebar to switch terminals. Multiple sessions can share a Pod, subject to its configured capacity. Sessions using the same local folder can edit the same files.

Use the [cloud acceptance checklist](model-compatibility.md) to validate the first model. Do not mark support from a text response alone.

## Sessions and credentials

App sessions use their own configuration and conversation histories under the app's data directory. Codex uses an app-owned `CODEX_HOME`, explicit provider settings at CLI precedence, and `--no-daemon`. Claude uses an app-owned `CLAUDE_CONFIG_DIR`, `--bare`, explicit gateway credentials/model mappings, empty setting sources, and strict MCP configuration.

Claude's bare mode intentionally skips normal plugins, hooks, project `CLAUDE.md` discovery, and subscription authentication. If project instructions are needed, explicitly ask the CLI to read the file as part of your task. Non-Claude inference remains experimental and unsupported by Anthropic. Disable this adapter in Setup if you do not want to use it.

The app does not modify normal CLI configuration or project provider files. It does not create Git branches, commits, pushes, or merges. A CLI still follows the instructions and permissions you give it; review its file/command approval requests.

**Resume** uses the original deployment/model and the CLI's actual history. Codex uses the latest conversation within that session's isolated state directory; Claude uses its recorded conversation ID. If the original Pod was terminated, create and enable a new deployment and open a new session. Moving an old conversation to another model is not supported.

Terminal scrollback is bounded, kept in memory, and disappears when the app exits. The session list and CLI-owned resumable histories persist. After a restart, old sessions are marked ended, their enabled idle cleanup is rearmed without extending existing countdowns, cloud state is reconciled, and saved routing is untrusted until verified again.

## Finish and quit

| Action | Result |
| --- | --- |
| Interrupt | Sends Ctrl+C to the local CLI; Pod continues running |
| Close session | Requests CLI/descendant termination and marks the session ended after cleanup; Pod continues unless idle cleanup is enabled |
| Close window | Hides the window; app and local cleanup timer continue |
| Finish deployment | Interrupts affected sessions, verifies owned Pod deletion, and waits for local tool cleanup |
| Finish and Quit | Waits for confirmed cleanup, stops local processes, exits |
| Leave Pod running and quit | Explicitly accepts ongoing costs; local cleanup timer stops |

**Cleanup Pending is not terminated.** Use Reconcile and the Runpod console until the provider confirms deletion. Retained network volumes continue billing after successful Pod deletion and must be managed separately in Runpod.

Maximum lifetime and budget controls are local best-effort timers. App sleep, loss of connectivity, API delays, or closing the app can exceed them. Provider-side scheduled termination was not reliable in the research recorded in [decisions](decisions.md). Do not leave an initial experiment unattended. A separate shutdown mechanism must be proven before claiming offline protection.

## Recovery

- **Unconfirmed creation:** Never create a replacement immediately. Reconcile. The app persists its unique ownership markers before the request and can find a Pod whose response was lost. If none is visible after at least two minutes, inspect the console and enter the exact saved Pod name in “Resolve an absent creation.” Late resources are still detected on later reconciliation.
- **Ownership mismatch:** Automatic deletion is refused. Inspect the saved Pod identity and account in the console. Never remove unrelated resources to clear the app warning.
- **Lost deletion response:** Reconcile. A successful DELETE response alone is insufficient; the app confirms that every tracked Pod is absent.
- **Authentication or wrong model:** New connected launches are blocked. Fatal readiness failures trigger cleanup. Inspect the preserved failure reason and model profile before recreating compute.
- **Download/access failure:** Check the model page and the token's read access. A 403 does not always distinguish pending approval from insufficient token permissions.
- **Temporary endpoint outage:** rate limits and gateway errors block new launches and trigger rechecking. They are not treated as proof of missing authentication. Initial startup remains bounded; previously ready deployments keep their lifetime/budget deadline while reconnecting.
- **Keychain or catalog unavailable:** saved deployments and recovery controls remain visible. Retry after restoring Keychain access, or import a valid catalog from Models. Older imported catalogs need the `minCacheGb` field; saved deployment profiles remain intact.
- **Cache too small:** choose a larger retained volume or temporary storage. Known-insufficient volume capacity is rejected before creation; actual free space is checked in the Pod, crediting complete files for the selected revision. An in-Pod failure still incurs startup charges until cleanup is confirmed.
- **Local cleanup unconfirmed:** keep the app open and retry Close. Finish can confirm Pod deletion while still reporting a local cleanup error; session processes must also stop before Quit succeeds. Process supervision runs only while the app is running.
- **GPU memory or runtime failure:** Keep the failed candidate unvalidated, finish cleanup, and adjust a new profile version before retrying.
- **Corrupt/incompatible local state:** The app refuses to overwrite it. Preserve the file and inspect/restore a known backup. Check Runpod directly for resources still billing.

## Protected Hugging Face models

The included profiles are public. For a gated/private profile, first accept its conditions and obtain access on Hugging Face. Create a read-scoped token and save it in **Runpod Secrets**. Enter only the secret's name in Setup. The app stores a reference and injects it only for protected profiles. It never needs to read your Hub token back.

Inside the Pod, the supervisor checks a protected weight file before starting the download. Revision downloads use the explicit model cache; authentication state is outside the retained cache. Cached files still need GPU loading and readiness verification. A retained volume must be large enough for the selected profile and is billed independently.

## Data and diagnostics

AI Deck retains the original installed-app identity for compatibility with saved state and credentials. App data is normally under `~/Library/Application Support/com.runpoddeck.desktop/`. It contains `state.json`, an optional imported `catalog.json`, and per-session CLI state. State writes are atomic and limited to one app instance. Protect backups of CLI histories: they can contain prompts, code, and command output.

Management and inference credentials use the Keychain service `com.runpoddeck.desktop`. State stores references only. The model container receives its inference credential and, when needed, the Hub secret reference; it never receives the Runpod management key.

**Export diagnostics** includes an explicit allowlist of deployment status, identities, profile fingerprints and timestamps. It excludes credentials, process environment, project paths, and terminal output. Raw CLI transcripts are not included. Avoid separately sharing upstream container logs without reviewing them.
