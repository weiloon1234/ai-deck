<script setup lang="ts">
import { nextTick, ref } from 'vue';
import type { ImportedModel, ModelImportResult } from '../../shared/contracts';
import ModelDetails from './ModelDetails.vue';
import ModelHardwareSuggestion from './ModelHardwareSuggestion.vue';
import ModelConfigurationEditor from './ModelConfigurationEditor.vue';
import UiIcon from '../../shared/UiIcon.vue';
import { useDeck } from '../../shared/useDeck';
const props = defineProps<{ model: ImportedModel }>();
const deck = useDeck();
const { modelImportBusy, modelImportNotice, selectedProfileId, view } = deck;
const editing = ref(false);
const editButton = ref<HTMLButtonElement | null>(null);
async function closeEditor() {
  editing.value = false;
  await nextTick();
  if (view.value === 'models') editButton.value?.focus();
}
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
    <div class="card-top"><span class="model-symbol"><UiIcon name="models" /></span><span class="tag">{{ model.experimentalConfiguration ? 'MANUAL CONFIGURATION' : model.profile ? 'AI SUGGESTION' : 'SAVED · NOT CONFIGURED' }}</span></div>
    <h2>{{ model.metadata.repository.split('/').pop() }}</h2>
    <p><a :href="`https://huggingface.co/${model.metadata.repository}`" target="_blank" rel="noreferrer" @click="deck.openExternal">{{ model.metadata.repository }} ↗</a></p>
    <p class="model-summary">{{ model.analysis.summary }}</p>
    <p v-if="!model.profile && !editing" class="small muted section-gap">{{ model.experimentalConfiguration ? 'Your experimental configuration has no launch profile at the current price or model requirements. Edit its settings to try another setup.' : 'No trial configuration yet. Edit the saved estimate to try this model experimentally.' }}</p>
    <details class="model-review"><summary>Model details, hardware and prices</summary><div class="model-overview"><ModelDetails :metadata="model.metadata" /><ModelHardwareSuggestion :model="model" /></div><button class="text-button section-gap" :disabled="!!modelImportBusy" @click="deck.modelOperation('Removing saved suggestion…', () => deck.call('remove_imported_model', { id: model.id }))">Remove from library</button></details>
    <ModelConfigurationEditor v-if="editing" :id="`configuration-${model.id}`" :model="model" @close="closeEditor" />
    <footer class="model-actions" v-if="!editing"><div class="button-row wrap"><button ref="editButton" class="button ghost" :disabled="!!modelImportBusy" @click="editing = true" :aria-controls="`configuration-${model.id}`" :aria-expanded="editing">Edit configuration</button><button class="button ghost" :disabled="!!modelImportBusy" @click="refreshPrices">Refresh prices</button><button class="text-button" :disabled="!!modelImportBusy" @click="reanalyze">Analyze again</button></div><button v-if="model.profile" class="button primary" :disabled="!!modelImportBusy" @click="selectedProfileId = model.id; view = 'deployments'">{{ model.experimentalConfiguration ? 'Try experimentally' : 'Use saved suggestion' }} <span>→</span></button></footer>
  </article>
</template>
