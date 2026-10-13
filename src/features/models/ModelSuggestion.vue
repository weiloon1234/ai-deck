<script setup lang="ts">
import type { ImportedModel, ModelImportResult } from '../../shared/contracts';
import ModelDetails from './ModelDetails.vue';
import ModelHardwareSuggestion from './ModelHardwareSuggestion.vue';
import UiIcon from '../../shared/UiIcon.vue';
import { useDeck } from '../../shared/useDeck';
const props = defineProps<{ model: ImportedModel }>();
const deck = useDeck();
const { modelImportBusy, modelImportNotice, selectedProfileId, view } = deck;
async function refreshPrices() {
  const result = await deck.modelOperation('Refreshing GPU prices…', async () => { await deck.call('refresh_model_prices', { id: props.model.id }); return true; });
  if (result) modelImportNotice.value = 'Current prices saved. Your Codex analysis was reused.';
}
async function reanalyze() {
  const result = await deck.modelOperation('Updating model suggestion…', () => deck.call<ModelImportResult>('import_hugging_face_model', { url: props.model.sourceUrl, reanalyze: true }));
  if (result) modelImportNotice.value = 'Updated model revision, analysis and prices saved. Existing deployments keep their original settings.';
}
</script>
<template>
  <article class="model-card imported-model">
    <div class="card-top"><span class="model-symbol"><UiIcon name="models" /></span><span class="tag amber">{{ model.profile ? 'AI SUGGESTION · UNTESTED' : 'SAVED · NEEDS REVIEW' }}</span></div>
    <h2>{{ model.metadata.repository.split('/').pop() }}</h2>
    <p><a :href="`https://huggingface.co/${model.metadata.repository}`" target="_blank" rel="noreferrer" @click="deck.openExternal">{{ model.metadata.repository }} ↗</a></p>
    <p class="model-summary">{{ model.analysis.summary }}</p>
    <details class="model-review"><summary>Model details, hardware and prices</summary><div class="model-overview"><ModelDetails :metadata="model.metadata" /><ModelHardwareSuggestion :model="model" /></div><button class="text-button section-gap" :disabled="!!modelImportBusy" @click="deck.modelOperation('Removing saved suggestion…', () => deck.call('remove_imported_model', { id: model.id }))">Remove from library</button></details>
    <footer class="model-actions"><div class="button-row wrap"><button class="button ghost" :disabled="!!modelImportBusy" @click="refreshPrices">Refresh prices</button><button class="button ghost" :disabled="!!modelImportBusy" @click="reanalyze">Analyze again</button></div><button v-if="model.profile" class="button primary" @click="selectedProfileId = model.id; view = 'deployments'">Use saved suggestion <span>→</span></button></footer>
  </article>
</template>
