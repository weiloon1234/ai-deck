<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue';
import type { ExperimentalModelConfiguration, Hardware, ImportedModel } from '../../shared/contracts';
import { money, useDeck } from '../../shared/useDeck';
const props = defineProps<{ model: ImportedModel }>();
const emit = defineEmits<{ close: [] }>();
const deck = useDeck();
const { state, modelImportBusy, modelImportNotice, selectedProfileId, view } = deck;
const savedVersion = props.model.profileVersion;
const a = props.model.analysis;
const configuration = ref<ExperimentalModelConfiguration>(props.model.experimentalConfiguration
  ? { ...props.model.experimentalConfiguration }
  : { gpuType: props.model.profile?.model.gpuTypes?.[0] ?? props.model.quotes[0]?.gpuType ?? '', gpuCount: a.gpuCount,
    minCpuRamGb: a.minCpuRamGb, diskGb: a.diskGb, minCacheGb: a.minCacheGb, contextTokens: a.recommendedContextTokens,
    toolParser: a.toolParser ?? null, reasoningParser: a.reasoningParser ?? null, quantization: a.quantization ?? null });
const hardware = ref<Hardware | null>(null);
const hardwareCheckedAt = ref<number | null>(null);
const loading = ref(false); const loadError = ref(''); const saveError = ref('');
const heading = ref<HTMLElement | null>(null);
let active = true;
onUnmounted(() => { active = false; });
const gpus = computed(() => hardware.value?.gpus.filter(g => g.secureCloud && g.memoryInGb > 0)
  .slice().sort((a, b) => (a.securePrice ?? Infinity) - (b.securePrice ?? Infinity)) ?? []);
const selectedGpu = computed(() => gpus.value.find(g => g.id === configuration.value.gpuType));
const rate = computed(() => {
  const price = selectedGpu.value?.securePrice;
  return price && Number.isFinite(price) && price > 0 && configuration.value.gpuCount > 0
    ? price * configuration.value.gpuCount : null;
});
const limit = computed(() => state.value?.settings.policy.maxHourlyUsd ?? 10);
const changed = computed(() => props.model.profileVersion !== savedVersion);
async function loadHardware() {
  if (loading.value || modelImportBusy.value) return;
  loading.value = true; loadError.value = '';
  const result = await deck.modelOperation('Reading GPU options…', () => deck.call<Hardware>('imported_model_hardware', { id: props.model.id }));
  if (active) {
    if (result && result.provider === (props.model.provider ?? 'runpod')) {
      hardware.value = result;
      hardwareCheckedAt.value = Date.now() / 1000;
      if (!configuration.value.gpuType) configuration.value.gpuType = gpus.value.find(g => g.securePrice && g.securePrice * configuration.value.gpuCount <= limit.value)?.id ?? '';
    } else loadError.value = deck.error?.value?.message ?? 'GPU options could not be loaded. Check your Runpod key in Setup and retry.';
  }
  loading.value = false;
}
const optionalName = (value: string | null) => value?.trim() || null;
async function tryExperimentally() {
  if (modelImportBusy.value || loading.value || changed.value || !rate.value || rate.value > limit.value) return;
  saveError.value = '';
  const c = configuration.value;
  const result = await deck.modelOperation('Saving experimental configuration…', async () => {
    await deck.call('configure_imported_model', { request: { id: props.model.id, expectedProfileVersion: savedVersion,
      configuration: { ...c, toolParser: optionalName(c.toolParser), reasoningParser: optionalName(c.reasoningParser), quantization: optionalName(c.quantization) } } });
    return true;
  });
  if (!active) return;
  if (result) {
    modelImportNotice.value = 'Experimental configuration saved. No compute was created. Review availability and costs in Deployment.';
    if (view.value === 'models') {
      selectedProfileId.value = props.model.id;
      view.value = (state.value?.settings.provider ?? 'runpod') === (props.model.provider ?? 'runpod') ? 'deployments' : 'setup';
    }
    emit('close');
  } else saveError.value = deck.error?.value?.message ?? 'Configuration was not saved. Review your settings and retry.';
}
onMounted(() => { heading.value?.focus(); void loadHardware(); });
</script>
<template>
  <form class="model-configuration" @submit.prevent="tryExperimentally" :aria-busy="loading || !!modelImportBusy">
    <div class="panel-heading"><div><h3 ref="heading" tabindex="-1">Experimental configuration</h3><p>Start from the saved estimate and adjust it for one coding session.</p></div><button class="text-button" type="button" :disabled="!!modelImportBusy && !loading" @click="emit('close')">Cancel</button></div>
    <p class="small muted configuration-identity">{{ model.metadata.repository }} · revision {{ model.metadata.revision.slice(0, 12) }} · vLLM {{ model.runtime.version }}</p>
    <p v-if="!model.analysis.runtimeCompatible" class="notice warning"><strong>Codex is unsure.</strong> {{ model.analysis.compatibilityReason }} You can still configure an experimental trial.</p>
    <p v-if="!model.metadata.configJson" class="notice" role="status"><strong>Model config wasn't read locally.</strong> You can still save a trial; architecture and published context remain unknown. <template v-if="model.metadata.access !== 'public'">Before creating a Pod, accept this model’s access conditions on Hugging Face, save a read-scoped token in Runpod Secrets, and enter only its secret name in Setup. Local metadata reads cannot use that secret.</template><template v-else>The runtime will try to retrieve the pinned config during startup.</template></p>
    <p v-if="changed" class="notice warning" role="status">This saved model changed while you were editing. Cancel and reopen Edit configuration to load the latest settings.</p>
    <p v-if="(state?.settings.provider ?? 'runpod') !== (model.provider ?? 'runpod')" class="notice">This model uses Runpod. After saving, choose Runpod in Setup to prepare its deployment.</p>
    <p v-if="loading" class="small muted section-gap" role="status">Loading GPU options and current prices…</p>
    <div v-if="loadError" class="notice" role="alert">{{ loadError }} <button type="button" class="inline-button" :disabled="!!modelImportBusy" @click="loadHardware">Retry</button></div>
    <div class="form-grid section-gap">
      <label>GPU type<select v-model="configuration.gpuType" required :disabled="loading || !!modelImportBusy"><option value="">Choose a GPU</option><option v-if="configuration.gpuType && !selectedGpu" :value="configuration.gpuType" disabled>{{ configuration.gpuType }} · unavailable</option><option v-for="gpu in gpus" :key="gpu.id" :value="gpu.id">{{ gpu.displayName }} · {{ gpu.memoryInGb }} GB{{ gpu.securePrice ? ` · ${money(gpu.securePrice)}/h each` : ' · no quote' }}</option></select></label>
      <label>GPU count<input v-model.number="configuration.gpuCount" type="number" min="1" max="8" step="1" required :disabled="!!modelImportBusy" /></label>
      <label>Context tokens<input v-model.number="configuration.contextTokens" type="number" min="1024" max="2097152" step="1" required :disabled="!!modelImportBusy" /></label>
      <label>Minimum CPU memory (GB)<input v-model.number="configuration.minCpuRamGb" type="number" min="1" max="16384" step="1" required :disabled="!!modelImportBusy" /></label>
      <label>Temporary model disk (GB)<input v-model.number="configuration.diskGb" type="number" min="20" max="2000" step="1" required :disabled="!!modelImportBusy" /></label>
      <label>Required cache capacity (GB)<input v-model.number="configuration.minCacheGb" type="number" min="1" :max="configuration.diskGb" step="1" required :disabled="!!modelImportBusy" /></label>
    </div>
    <p class="small muted section-gap">{{ model.metadata.weightSizeGb ? `Reported weights: ${model.metadata.weightSizeGb.toFixed(1)} GB. ` : '' }}Cache needs the full weights plus working space. Context and GPU capacity are untested.</p>
    <details class="advanced"><summary>Optional runtime settings</summary><p class="small muted section-gap">Use built-in names supported by this vLLM runtime. Empty fields omit the flag; paths, commands and extra flags are not accepted.</p><div class="form-grid section-gap">
      <label>Tool-call parser<input v-model="configuration.toolParser" maxlength="64" placeholder="Leave empty or enter a parser name" :disabled="!!modelImportBusy" autocapitalize="off" spellcheck="false" /></label>
      <label>Reasoning parser<input v-model="configuration.reasoningParser" maxlength="64" placeholder="Leave empty or enter a parser name" :disabled="!!modelImportBusy" autocapitalize="off" spellcheck="false" /></label>
      <label>Quantization<input v-model="configuration.quantization" maxlength="64" placeholder="Leave empty to use model defaults" :disabled="!!modelImportBusy" autocapitalize="off" spellcheck="false" /></label>
    </div></details>
    <p v-if="saveError" class="notice warning" role="alert">{{ saveError }}</p>
    <footer class="panel-footer"><div><strong>{{ rate ? `${money(rate)}/hour for ${configuration.gpuCount} GPU(s)` : 'Choose a GPU with a current price' }}</strong><small v-if="hardwareCheckedAt">GPU prices checked {{ new Date(hardwareCheckedAt * 1000).toLocaleString() }}</small><small :class="{ 'amber-text': rate && rate > limit }">{{ rate && rate > limit ? 'Over your hourly limit. Choose fewer GPUs or a different GPU.' : `GPU limit ${money(limit)}/hour · storage extra` }}</small><small>Next: review Deployment. Saving does not create paid compute.</small></div><button class="button primary" type="submit" :disabled="!!modelImportBusy || loading || changed || !rate || rate > limit">Try experimentally →</button></footer>
  </form>
</template>
