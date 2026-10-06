<script setup lang="ts">
import { computed, ref } from 'vue';
import { useDeck } from '../../shared/useDeck';
import type { ModelImportResult } from '../../shared/contracts';
import ModelSuggestion from './ModelSuggestion.vue';
const deck = useDeck();
const { snapshot, state, selectedProfileId, view, perform, call, busy, modelImportBusy, modelImportNotice } = deck;
const url = ref('');
const search = ref('');
const matches = (text: string) => text.toLowerCase().includes(search.value.trim().toLowerCase());
const imported = computed(() => state.value?.importedModels.filter(m => matches(m.metadata.repository)) ?? []);
const profiles = computed(() => snapshot.value?.catalog.filter(p => !state.value?.importedModels.some(m => m.id === p.model.id) && matches(`${p.model.displayName} ${p.model.repository}`)) ?? []);
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
  <div class="page-heading"><div><div class="eyebrow">MODEL LIBRARY</div><h1>Bring your own coding model.</h1><p>Paste a Hugging Face link. Save a hardware suggestion for next time.</p></div><button class="button ghost" :disabled="!!busy || !!modelImportBusy" @click="perform('Importing catalog', () => call('import_catalog'))">Import catalog file</button></div>
  <form class="panel model-import" @submit.prevent="importModel">
    <div class="panel-heading"><div><h2>Add a model from Hugging Face</h2><p>Your local Codex subscription analyzes hardware for one coding session and a usable context range.</p></div><span class="tag green">SAVED LOCALLY</span></div>
    <label for="model-url">Hugging Face model URL</label>
    <div class="input-action section-gap"><input id="model-url" v-model="url" type="url" required placeholder="https://huggingface.co/organization/model" :disabled="!!modelImportBusy" autocomplete="off" /><button class="button primary" type="submit" :disabled="!!modelImportBusy || !url.trim()">Analyze &amp; save</button></div>
    <p class="small muted">Uses your signed-in Codex CLI and reads current Runpod GPU prices. Importing does not create a Pod. Public model pages and /tree/revision links are supported.</p>
    <p v-if="!snapshot?.runpodKeyConfigured" class="notice">Add your Runpod API key in <button class="inline-button" type="button" @click="view = 'setup'">Setup</button> to include current pricing.</p>
    <div v-if="modelImportBusy" class="import-progress" role="status"><span class="spinner" />{{ modelImportBusy }}<button class="text-button push-right" type="button" @click="call('cancel_model_import').catch(deck.report)">Cancel</button></div>
    <p v-if="modelImportNotice" class="notice" role="status">{{ modelImportNotice }}</p>
  </form>
  <div class="notice">Hardware and context ranges are suggestions for getting the model running. They have not been measured on a paid GPU. Price snapshots exclude storage; launch checks current pricing and your budget again.</div>
  <label class="model-search">Find a saved model<input v-model="search" type="search" placeholder="Search model name or organization" /></label>
  <div v-if="imported.length" class="imported-model-list section-gap">
    <ModelSuggestion v-for="model in imported" :key="model.id" :model="model" />
  </div>
  <h2 v-if="profiles.length" class="section-gap">Catalog profiles</h2>
  <div class="model-grid">
    <article v-for="profile in profiles" :key="profile.model.id" class="model-card section-gap">
      <div class="card-top"><span class="model-symbol">{{ profile.model.id.startsWith('gpt') ? '◎' : 'Q' }}</span><span class="tag amber">{{ profile.model.evidence.length ? 'EVIDENCE ATTACHED' : 'NOT YET TESTED' }}</span></div>
      <h2>{{ profile.model.displayName }}</h2><p>{{ profile.model.repository }}</p>
      <div class="model-metrics"><div><strong>{{ (profile.model.contextTokens / 1024).toFixed(0) }}k</strong><span>Configured context</span></div><div><strong>{{ profile.model.maxSessions }}</strong><span>Configured sessions</span></div><div><strong>{{ profile.model.minGpuMemoryGb }} GB</strong><span>GPU memory</span></div></div>
      <div class="protocols"><span>Codex · Responses</span><span v-if="profile.model.supportsMessages">Claude Code · Experimental</span></div>
      <ul class="limitations"><li v-for="limit in profile.model.limitations" :key="limit">{{ limit }}</li></ul>
      <details class="advanced"><summary>Version and validation details</summary><dl><dt>Model revision</dt><dd>{{ profile.model.revision }}</dd><dt>Runtime</dt><dd>vLLM {{ profile.runtime.version }}</dd><dt>Image digest</dt><dd>{{ profile.runtime.image }}</dd><dt>Configuration fingerprint</dt><dd>{{ profile.fingerprint }}</dd><dt>Evidence</dt><dd>{{ profile.model.evidence.length }} reports. Each launch checks the exact CLI version and required capabilities.</dd></dl></details>
      <button class="button primary wide" @click="selectedProfileId = profile.model.id; view = 'deployments'">Use this profile <span>→</span></button>
    </article>
  </div>
  <p v-if="!imported.length && !profiles.length" class="notice">No saved models match your search.</p>
</template>
