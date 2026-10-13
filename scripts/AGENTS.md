# Diagnostic and preparation script guidance

Inherits [root guidance](../AGENTS.md). These tools are opt-in diagnostics or local preparation, separate from production provider adapters.

- Keep probes read-only and limited to documented endpoints. No provisioning, image/model downloads, retries of writes or paid inference. The regular checks must never run an authenticated cloud probe.
- Use Python's standard library and existing tests. Keep service hosts fixed, reject redirects, disable ambient proxies, bound reads/timeouts and sanitize errors.
- Working keys must be entered interactively without echo and never passed in command-line arguments, written to files, or included in results. Do not read saved app credentials or state implicitly. Print observations/counts rather than upstream records.
- A marker readback observation does not enable a provider or establish server idempotency. Preserve that distinction in output and documentation. Tests use fake replies/keys and belong under `tests/runtime/`.
- Runtime-image preparation reads the catalog and supervisor/recipe sources, prints structured build arguments and never invokes Docker or a registry. Keep base-image digests in the catalog and include source changes in recipe identity. Publishing or deploying still needs an immutable registry digest and the documented provider gates.

Read [provider integration](../docs/provider-integration.md) before using the GPUtw probe. It does not authorize creating a test instance. Only run it against an existing owner-selected record after the owner supplies the scoped key in their terminal.
