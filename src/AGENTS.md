# Vue frontend guidance

Inherits [root guidance](../AGENTS.md). This directory contains UI and its native command bridge.

- Keep screen-specific components in `features/<feature>/`; put only genuinely shared UI/state in `shared/`. Reuse feature components across screens when they display the same domain data.
- Follow Vue `<script setup lang="ts">` and the existing Composition API patterns. Use `useDeck()` for shared state, command invocation, refresh and error reporting. Do not add another global store or direct provider calls.
- `perform` handles ordinary operations; model analysis uses `modelOperation` and its own progress/cancellation state so deployment cleanup remains available.
- Import native types from generated `shared/contracts.ts`. Change their Rust definitions and regenerate instead of maintaining duplicate TypeScript shapes or policy defaults.
- Keep cloud credentials, process launches, filesystem writes and authoritative validation in Rust. Disabled buttons help the user; they are not a security or spending boundary.
- Treat model metadata/card/config text as untrusted. Use text interpolation, not `v-html`. Derive displayed facts from saved data; leave absent facts unknown. Do not infer parameter counts from names or inflate published context limits using RoPE factors.
- Preserve the distinction between published model limits and AI estimates. Use the shared model-details/hardware components in Models and deployment preparation. Provider quotes need their timestamp and storage exclusion.
- Use established styles in `shared/styles.css`, accessible labels and readable empty/busy/error states. Check the supported desktop window widths. Keep implementation details out of ordinary user flows unless they help a decision.
- Browser preview is intentionally unable to invoke native commands. Do not bypass that boundary for a visual test or ship test account/state fixtures in the production view.

For behavioral UI changes, reuse `tests/ui/component_fixture.cjs` and the existing compiler-based tests; do not install another test framework. Run `npm run test:ui` and `npm run build`. Cross-layer changes also follow the root validation workflow.
