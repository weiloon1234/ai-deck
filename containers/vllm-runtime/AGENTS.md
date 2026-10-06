# GPU runtime supervisor guidance

Inherits [root guidance](../../AGENTS.md). Read [README.md](README.md) for the runtime's role.

- Keep `launcher.py` a focused supervisor using the pinned upstream image. The image/version and model settings belong in `model-catalog/catalog.json`, not another Docker image or duplicate constants here.
- The provisioner embeds this file and launches it with structured arguments. Preserve explicit argv construction; never introduce shell interpolation of model data or credentials.
- Access checks, cache-capacity checks, immutable-revision downloads, startup timeouts and authenticated status must remain enforced before readiness. Keep model and compilation caches separate and compilation caches tied to the profile fingerprint.
- Preserve temporary authentication storage and the existing Runpod Secret route for gated downloads. Never pass the Runpod management key to the container or log credentials/raw provider errors.
- Expose only sanitized diagnostic categories. Inference stays with vLLM's authenticated API; do not add a response-translation layer or remote-code execution without an explicit approved design change.
- Runtime exit is not cloud resource deletion. Preserve diagnostic status for failed startup and leave Pod ownership/deletion to the native controller. Never delete retained network volumes.
- Supervisor bytes contribute to the resolved profile fingerprint. Changes can invalidate compatibility evidence; never copy evidence to a new fingerprint without the corresponding real validation.

Use `python3 -m unittest discover -s tests/runtime -v` with mocked downloads. Do not pull images, download weights, start GPU compute or perform live inference as a test without explicit authorization. Run the root cross-layer checks when changing the Rust/runtime contract.
