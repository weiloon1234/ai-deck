# Documentation and evidence guidance

Inherits [root guidance](../AGENTS.md). Documentation must describe the implementation and distinguish local checks from owner-led live acceptance.

- README explains what the app does, why its architecture exists, how to build/use it and where to find detailed guidance. Keep operating steps in `operating-guide.md`, model import behavior in `model-import.md` and live evidence requirements in `model-compatibility.md`.
- Record current review/test results with a date and concrete commands in `local-validation.md`. Preserve earlier results as historical evidence; do not present an old passing run as a current run or overwrite the original re-audit findings.
- `decisions.md` and the project plan record approved scope and historical research. Qualify dated provider/tool observations instead of silently treating them as current guarantees. Check primary documentation when making a new external behavior claim.
- Model facts, AI estimates, provider quotes and measured compatibility are separate evidence. The owner removed visible candidate-test warnings and acknowledgements; preserve measured evidence and unknown compatibility facts in documentation/details without restoring those UI gates. Fixture results never prove a successful coding model.
- Document cost/cleanup limitations beside the relevant flow. Adding credits, saving a key or enabling provisioning is different from confirmed paid creation. Pod deletion and retained-storage cleanup are separate actions.
- Link to authoritative catalog, Rust checks and developer scripts rather than copying changing constants or command chains. Keep relative repository links valid and free of machine-specific paths unless a historical observation requires one.
- Owner compatibility reports belong in `compatibility-reports/` after actual testing. Record exact fingerprints, versions, hardware, observations and failures; exclude secrets, unredacted headers and private account data.

Documentation-only edits need source/link/diff review. When documenting code changes, record the checks actually run and any unverified release gates.
