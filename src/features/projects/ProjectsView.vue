<script setup lang="ts">
import { useDeck } from '../../shared/useDeck';
defineEmits<{ launch: [projectId: string] }>();
const { state, perform, call, busy } = useDeck();
</script>
<template>
  <div class="page-heading"><div><div class="eyebrow">ON YOUR MAC</div><h1>Your projects.</h1><p>Files, commands, and tests run in the folder you select.</p></div><button class="button primary" :disabled="!!busy" @click="perform('Adding project', () => call('choose_project'))">+ Add project</button></div>
  <div v-if="!state?.projects.length" class="empty-card"><div class="empty-symbol">⌑</div><h2>Start with a local folder.</h2><p>Add an existing project. AI Deck does not create branches or change your Git history.</p><button class="button ghost" :disabled="!!busy" @click="perform('Adding project', () => call('choose_project'))">Choose project folder</button></div>
  <div v-else class="project-list"><article v-for="project in state.projects" :key="project.id" class="project-row"><span class="folder-icon">⌑</span><div class="grow"><h3>{{ project.name }}</h3><p class="path">{{ project.path }}</p><span class="muted small">{{ state.sessions.filter(s => s.projectId === project.id && s.status !== 'ended').length }} active sessions in this folder</span></div><button class="button ghost" @click="$emit('launch', project.id)">Open CLI</button><button class="text-button" :disabled="!!busy" @click="perform('Removing project from list', () => call('remove_project', { id: project.id }))">Remove</button></article></div>
  <p class="footnote">Removing a project only removes it from this list. Sessions sharing a folder can edit the same files.</p>
</template>
