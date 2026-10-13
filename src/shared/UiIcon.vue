<script setup lang="ts">
const paths = {
  deployments: 'M4 4h16v6H4z M4 14h16v6H4z M8 7h.01 M8 17h.01',
  models: 'm12 3 9 5-9 5-9-5 9-5Z M3 12l9 5 9-5 M3 16l9 5 9-5',
  projects: 'M3 7V5h6l2 2h10v12H3V7Z',
  sessions: 'M3 4h18v16H3z m4 5 3 3-3 3 M13 15h4',
  setup: 'M4 7h16 M4 17h16 M8 4v6 M16 14v6',
  clock: 'M12 8v4l3 2 M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0',
} as const;
defineProps<{ name: keyof typeof paths }>();
</script>
<template>
  <svg class="ui-icon" viewBox="0 0 24 24" aria-hidden="true"><path :d="paths[name]" /></svg>
</template>
