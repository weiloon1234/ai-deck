<script setup lang="ts">
import { computed } from 'vue';
import type { HubModel } from '../../shared/contracts';
import { useDeck } from '../../shared/useDeck';
const props = defineProps<{ metadata: HubModel }>();
const deck = useDeck();
type Config = Record<string, unknown>;
const object = (value: unknown): Config => value && typeof value === 'object' && !Array.isArray(value) ? value as Config : {};
const label = (value: unknown): string | undefined => typeof value === 'string' && value.trim() ? value.trim() : undefined;
const positive = (value: unknown): number | undefined => typeof value === 'number' && Number.isSafeInteger(value) && value > 0 ? value : undefined;
const config = computed(() => {
  try { return object(JSON.parse(props.metadata.configJson)); } catch { return {}; }
});
// Multimodal configs keep language limits under text_config; vision limits are unrelated.
const textConfig = computed(() => Object.keys(object(config.value.text_config)).length ? object(config.value.text_config) : config.value);
const context = computed(() => {
  for (const key of ['max_position_embeddings', 'n_positions', 'max_seq_len', 'max_sequence_length', 'seq_length']) {
    const value = positive(textConfig.value[key]);
    if (value) return { value, key: `${textConfig.value === config.value ? '' : 'text_config.'}${key}` };
  }
  return undefined;
});
const parameterCount = computed(() => positive(props.metadata.parameterCount));
const parameters = computed(() => {
  const n = parameterCount.value;
  if (!n) return 'Not provided';
  const unit = n >= 1e12 ? { size: 1e12, name: 'T' } : n >= 1e9 ? { size: 1e9, name: 'B' } : n >= 1e6 ? { size: 1e6, name: 'M' } : undefined;
  return unit ? `${(n / unit.size).toLocaleString(undefined, { maximumFractionDigits: 2 })}${unit.name}` : n.toLocaleString();
});
const architecture = computed(() => {
  const names = config.value.architectures;
  return Array.isArray(names) ? names.map(label).filter(Boolean).join(', ') || undefined : undefined;
});
const dtype = computed(() => label(textConfig.value.dtype) ?? label(textConfig.value.torch_dtype) ?? label(config.value.dtype) ?? label(config.value.torch_dtype));
const quantization = computed(() => {
  const q = object(textConfig.value.quantization_config ?? config.value.quantization_config);
  if (!Object.keys(q).length) return 'Not specified';
  const method = label(q.quant_method) ?? 'See saved config';
  const bits = positive(q.bits);
  return `${method}${bits ? ` · ${bits}-bit` : ''}`;
});
const structure = computed(() => {
  const cfg = textConfig.value;
  const fields: [string, unknown][] = [
    ['Layers', cfg.num_hidden_layers ?? cfg.n_layer ?? cfg.num_layers],
    ['Attention heads', cfg.num_attention_heads ?? cfg.n_head ?? cfg.n_heads],
    ['Key/value heads', cfg.num_key_value_heads], ['Hidden size', cfg.hidden_size ?? cfg.n_embd ?? cfg.d_model],
    ['Vocabulary size', cfg.vocab_size], ['Experts', cfg.num_local_experts ?? cfg.num_experts ?? cfg.n_routed_experts],
    ['Experts per token', cfg.num_experts_per_tok],
  ];
  return fields.flatMap(([name, value]) => positive(value) ? [{ name, value: (value as number).toLocaleString() }] : []);
});
const rope = computed(() => {
  const value = object(textConfig.value.rope_scaling);
  return Object.keys(value).length ? JSON.stringify(value, null, 2) : undefined;
});
const prettyConfig = computed(() => JSON.stringify(config.value, null, 2));
</script>
<template>
  <section class="model-details">
    <div class="eyebrow">HUGGING FACE · SAVED METADATA</div><h3>Model details</h3>
    <div class="model-metrics"><div><strong :title="parameterCount?.toLocaleString()">{{ parameters }}</strong><span>Parameters reported by Hugging Face</span></div></div>
    <div class="context-range model-context"><strong>{{ context ? `${context.value.toLocaleString()} tokens` : 'Not provided' }}</strong><span>Published context limit · saved config</span></div>
    <p class="small muted section-gap">Input and output share the context window. The usable range on your GPU may be lower. Scaling settings are shown separately.</p>
    <dl class="model-facts">
      <dt>Architecture</dt><dd>{{ architecture ?? 'Not provided' }}</dd>
      <dt>Model type</dt><dd>{{ label(textConfig.model_type) ?? label(config.model_type) ?? 'Not provided' }}</dd>
      <dt>Precision</dt><dd>{{ dtype ?? 'Not provided' }}</dd>
      <dt>Quantization</dt><dd>{{ quantization }}</dd>
      <dt>Weight files</dt><dd>{{ metadata.weightFormat ?? 'Format unavailable' }} · {{ metadata.weightSizeGb == null ? 'Size unavailable' : `${metadata.weightSizeGb.toFixed(1)} GB download` }}{{ metadata.isAdapter ? ' · adapter repository' : '' }}</dd>
      <dt>Task / library</dt><dd>{{ metadata.pipelineTag ?? 'Not provided' }} / {{ metadata.libraryName ?? 'Not provided' }}</dd>
      <dt>License</dt><dd>{{ metadata.license ?? 'Not provided' }}</dd>
      <dt>Access</dt><dd>{{ metadata.access }}</dd>
    </dl>
    <details class="advanced"><summary>Architecture details and source</summary>
      <dl><template v-for="fact in structure" :key="fact.name"><dt>{{ fact.name }}</dt><dd>{{ fact.value }}</dd></template>
        <dt v-if="parameterCount">Reported parameters</dt><dd v-if="parameterCount">{{ parameterCount.toLocaleString() }} · Hub safetensors total</dd>
        <dt v-if="context">Context field</dt><dd v-if="context">{{ context.key }}</dd>
        <dt v-if="positive(textConfig.sliding_window)">Sliding window</dt><dd v-if="positive(textConfig.sliding_window)">{{ Number(textConfig.sliding_window).toLocaleString() }}{{ textConfig.use_sliding_window === false ? ' · disabled in config' : ' · architecture setting' }}</dd>
        <dt>Saved revision</dt><dd>{{ metadata.revision }}</dd>
      </dl>
      <p class="small muted">Counts describe the stored tensors and may differ from original or active parameters for quantized, adapter or mixture-of-experts models. Missing values are not inferred from the model name.</p>
      <div v-if="rope" class="section-gap"><h3>Context scaling (RoPE)</h3><pre class="model-config">{{ rope }}</pre></div>
      <a :href="`https://huggingface.co/${metadata.repository}/blob/${metadata.revision}/config.json`" target="_blank" rel="noreferrer" @click="deck.openExternal">View pinned config on Hugging Face ↗</a>
      <details v-if="Object.keys(config).length" class="advanced"><summary>Saved config</summary><pre class="model-config">{{ prettyConfig }}</pre></details>
      <ul v-if="metadata.notes.length" class="limitations"><li v-for="(note, index) in metadata.notes" :key="index">{{ note }}</li></ul>
    </details>
  </section>
</template>
