<script setup lang="ts">
import { computed, ref } from 'vue';
import { useDeck } from '../../shared/useDeck';
import type { ModelImportResult } from '../../shared/contracts';
import ModelSuggestion from './ModelSuggestion.vue';
import UiIcon from '../../shared/UiIcon.vue';
const deck = useDeck();
const { snapshot, state, selectedProfileId, view, perform, call, busy, modelImportBusy, modelImportNotice } = deck;
const url = ref('');
const search = ref('');
const matches = (text: string) => text.toLowerCase().includes(search.value.trim().toLowerCase());
const imported = computed(() => state.value?.importedModels.filter(m => matches(m.metadata.repository)) ?? []);
const catalogProfiles = computed(() => snapshot.value?.catalog.filter(p => !state.value?.importedModels.some(m => m.id === p.model.id)) ?? []);
const profiles = computed(() => catalogProfiles.value.filter(p => matches(`${p.model.displayName} ${p.model.repository}`)));
const libraryCount = computed(() => (state.value?.importedModels.length ?? 0) + catalogProfiles.value.length);
const importAvailable = computed(() => (state.value?.settings.provider ?? 'runpod') === 'runpod');
async function importModel() {
  if (!url.value.trim()) return;
  const result = await deck.modelOperation('Reading model details…', () => call<ModelImportResult>('import_hugging_face_model', { url: url.value.trim(), reanalyze: false }));
  if (result) {
    modelImportNotice.value = result.reused ? 'Loaded the saved suggestion. No new Codex analysis was needed. Refresh prices for a current quote.' : 'Model and suggestions saved. You can reuse them next time.';
    url.value = ''; search.value = '';
  }
}
</script>
<template>
  <div class="page-heading"><div><h1>Models</h1><p>Your saved models and hardware suggestions, ready to reuse.</p></div><button class="button ghost" :disabled="!!busy || !!modelImportBusy" @click="perform('Importing catalog', () => call('import_catalog'))">Import catalog file</button></div>
  <form class="panel model-import" @submit.prevent="importModel">
    <div class="panel-heading"><div><h2>Add a model</h2><p>Codex suggests hardware and usable context for one coding session.</p></div></div>
    <label for="model-url">Hugging Face model URL</label>
    <div class="input-action import-input"><input id="model-url" v-model="url" type="url" required placeholder="https://huggingface.co/organization/model" :disabled="!!modelImportBusy" autocomplete="off" /><button class="button primary" type="submit" :disabled="!!modelImportBusy || !url.trim() || !importAvailable">Analyze &amp; save</button></div>
    <p v-if="!importAvailable" class="notice">GPUtw hardware analysis is pending. Choose Runpod in <button class="inline-button" type="button" @click="view = 'setup'">Setup</button> to analyze new models. Saved Runpod suggestions remain available.</p>
    <p class="small muted">Uses your signed-in Codex CLI and reads current Runpod GPU prices. Importing does not create a Pod. Public model pages and /tree/revision links are supported.</p>
    <p v-if="!snapshot?.runpodKeyConfigured" class="import-setup-note">Add your Runpod API key in <button class="inline-button" type="button" @click="view = 'setup'">Setup</button> to include current pricing.</p>
    <div v-if="modelImportBusy" class="import-progress" role="status"><span class="spinner" />{{ modelImportBusy }}<button class="text-button push-right" type="button" @click="call('cancel_model_import').catch(deck.report)">Cancel</button></div>
    <p v-if="modelImportNotice" class="notice" role="status">{{ modelImportNotice }}</p>
  </form>
  <div class="library-toolbar"><h2>Model library <span class="library-count">{{ libraryCount }}</span></h2><label class="model-search"><span class="visually-hidden">Find a saved model</span><input v-model="search" type="search" placeholder="Search models or organizations" /></label></div>
  <div v-if="imported.length" class="imported-model-list section-gap">
    <ModelSuggestion v-for="model in imported" :key="model.id" :model="model" />
  </div>
  <h3 v-if="imported.length && profiles.length" class="library-section-label">Catalog profiles</h3>
  <div class="model-grid">
    <article v-for="profile in profiles" :key="profile.model.id" class="model-card">
      <div class="card-top"><span class="model-symbol"><UiIcon name="models" /></span><span v-if="profile.model.evidence.length" class="tag">EVIDENCE ATTACHED</span></div>
      <h2>{{ profile.model.displayName }}</h2><p>{{ profile.model.repository }}</p>
      <div class="model-metrics"><div><strong>{{ (profile.model.contextTokens / 1024).toFixed(0) }}k</strong><span>Configured context</span></div><div><strong>{{ profile.model.maxSessions }}</strong><span>Configured sessions</span></div><div><strong>{{ profile.model.minGpuMemoryGb }} GB</strong><span>GPU memory</span></div></div>
      <div class="protocols"><span>Codex · Responses</span><span v-if="profile.model.supportsMessages">Claude Code · Experimental</span></div>
      <details class="advanced"><summary>Limitations, version and validation</summary><ul class="limitations"><li v-for="limit in profile.model.limitations" :key="limit">{{ limit }}</li></ul><dl><dt>Model revision</dt><dd>{{ profile.model.revision }}</dd><dt>Runtime</dt><dd>vLLM {{ profile.runtime.version }}</dd><dt>Image digest</dt><dd>{{ profile.runtime.image }}</dd><dt>Configuration fingerprint</dt><dd>{{ profile.fingerprint }}</dd><dt>Evidence</dt><dd>{{ profile.model.evidence.length }} reports, matched to the exact CLI version and configured capacity.</dd></dl></details>
      <button class="button primary wide" @click="selectedProfileId = profile.model.id; view = 'deployments'">Use this profile <span>→</span></button>
    </article>
  </div>
  <div v-if="!imported.length && !profiles.length" class="notice search-empty" role="status"><span>No saved models match your search.</span><button v-if="search" class="button ghost compact" @click="search = ''">Clear search</button></div>
  <p class="footnote">GPU price snapshots exclude storage. Deployment checks current pricing and your budget again.</p>
</template>
