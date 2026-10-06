<script setup lang="ts">
import { computed } from 'vue';
import type { ImportedModel } from '../../shared/contracts';
import { money, useDeck } from '../../shared/useDeck';
const props = defineProps<{ model: ImportedModel }>();
const { state } = useDeck();
const limit = computed(() => state.value?.settings.policy.maxHourlyUsd ?? 10);
const withinBudget = computed(() => props.model.quotes.some(q => q.hourlyUsd !== null && q.hourlyUsd <= limit.value));
const date = (at: number) => new Date(at * 1000).toLocaleString();
const tokens = (value: number) => value.toLocaleString();
</script>
<template>
  <section class="model-hardware">
    <div class="eyebrow">CODEX · UNTESTED ESTIMATE</div><h3>Suggested Runpod setup</h3>
    <p>{{ model.analysis.summary }}</p>
    <div class="model-metrics"><div><strong>{{ model.analysis.minimumTotalVramGb }} GB</strong><span>Suggested total GPU memory</span></div><div><strong>{{ model.analysis.gpuCount }}</strong><span>GPU(s) · one coding session</span></div></div>
    <div class="context-range"><strong>{{ tokens(model.analysis.contextMinTokens) }}–{{ tokens(model.analysis.contextMaxTokens) }} tokens</strong><span>Suggested usable context · default {{ tokens(model.analysis.recommendedContextTokens) }}</span></div>
    <div class="quote-list">
      <h3>Runpod GPU price snapshot</h3>
      <p class="small muted">Checked {{ date(model.pricesCheckedAt) }} · storage extra</p>
      <div v-for="quote in model.quotes" :key="quote.gpuType" class="quote-row">
        <div><strong>{{ quote.gpuCount }} × {{ quote.gpuType }}</strong><small>{{ quote.availableRegions.length ? `Stock listed in ${quote.availableRegions.join(', ')}` : 'No region currently lists stock' }} · availability checked at launch</small></div>
        <div class="quote-price"><strong>{{ quote.hourlyUsd === null ? 'No quote' : `${money(quote.hourlyUsd)}/h` }}</strong><small v-if="quote.hourlyUsd !== null && quote.hourlyUsd > limit" class="amber-text">Over {{ money(limit) }}/h limit</small></div>
      </div>
      <p v-if="!model.quotes.length" class="small muted">No matching Secure Cloud GPU quote. Reanalyze to consider other hardware.</p>
    </div>
    <p v-if="!model.analysis.runtimeCompatible" class="notice warning">{{ model.analysis.compatibilityReason }}</p>
    <p v-else-if="!model.profile" class="notice warning">{{ !withinBudget ? 'No priced configuration fits your GPU budget.' : 'This model needs supported full weights, enough cache space, a complete config and tool-calling setup before it can become a launch profile.' }} The analysis is saved for reference.</p>
    <details class="advanced"><summary>Assumptions and runtime requirements</summary>
      <p class="small section-gap">{{ model.analysis.compatibilityReason }}</p>
      <dl><dt>CPU memory</dt><dd>{{ model.analysis.minCpuRamGb }} GB suggested</dd><dt>Disk / cache</dt><dd>{{ model.analysis.diskGb }} GB / {{ model.analysis.minCacheGb }} GB</dd><dt>Runtime</dt><dd>vLLM {{ model.runtime.version }}</dd><dt>Analysis</dt><dd>{{ date(model.analyzedAt) }} · {{ model.analyzerVersion }}</dd></dl>
      <ul class="limitations"><li v-for="(note, index) in [...model.analysis.assumptions, ...model.analysis.warnings]" :key="index">{{ note }}</li></ul>
    </details>
  </section>
</template>
