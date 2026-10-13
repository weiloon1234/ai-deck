# Coding CLI adapter guidance

Inherits [native guidance](../../AGENTS.md). These launchers serve app-owned coding sessions; the normal-subscription model analyzer is a separate service.

- Reuse `LaunchSpec`, `controlled_environment`, executable validation and version discovery in `mod.rs`. Keep executable, arguments and environment structured; do not assemble a shell command.
- Preserve isolated per-session state, explicit endpoint/model/credential settings and normal approval controls. Critical routing settings must survive project/config precedence. Never inherit provider keys or fall back to subscription authentication after endpoint failure.
- Codex's saved provider ID is a compatibility contract. Claude's bare mode, setting-source restrictions, helper model routing and experimental status are deliberate isolation controls.
- The owner explicitly requested trial-and-error with installed versions. Do not reintroduce an exact-version allowlist for analysis, launch or resume. Record the detected version for compatibility evidence; do not claim it is validated automatically. Preserve isolation flags even when a CLI rejects them, and use the optional installed-CLI probes when changing those flags. No automatic CLI installation or upgrade.
- Launch/resume no longer require candidate-test acknowledgement. Preserve compatibility metadata without treating the removal as a passing test.
- Resume must use the session's original deployment/model and CLI history. Keep credentials out of generated configuration files and include endpoint tokens in terminal redaction values.

Run native CLI isolation and terminal regressions for adapter changes. Ignored installed-CLI probes are deliberate opt-in checks with fake keys and a localhost rejection server; they do not establish successful model inference. See [native test guidance](../../tests/AGENTS.md).
