# Model catalog guidance

Inherits [root guidance](../AGENTS.md). This directory defines bundled model/runtime candidates, not measured GPU compatibility.

- Edit `catalog.json` for bundled values. Native validation, launch arguments and fingerprint rules live in `src-tauri/src/model_catalog.rs`; reuse them instead of adding a parallel parser or deployment constants.
- `schema.json` is generated from Rust. Run `npm run contracts` after changing schema/type sources, then inspect all generated changes. Never hand-edit the schema.
- Keep model revisions immutable, container images pinned by digest and identifiers/cache paths validated. Tie parser/quantization settings to the pinned runtime and structured launch arguments; unsupported values need explicit validation. Do not enable remote model code or add executable launch fragments.
- Compatibility evidence must identify the tested fingerprint, exact CLI version and GPU. Read [compatibility guidance](../docs/model-compatibility.md); required checks come from Rust and depend on declared session capacity. Mock results, published metadata and AI estimates cannot become passing evidence.
- Launcher source is part of the fingerprint. Changes to the supervisor, runtime or model settings invalidate existing evidence; saved deployments retain their original profile snapshots.
- Imported suggestions belong in the existing state store through `model_import_service.rs`. Do not copy them into the bundled catalog automatically or silently retarget sessions.

Use the catalog/storage and model-import regressions, generated drift check and root verification workflow. Do not download weights or query paid resources to validate JSON edits.
