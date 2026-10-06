<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { useDeck } from '../../shared/useDeck';
import type { AppSettings, CliInstallation } from '../../shared/contracts';
const deck = useDeck();
const { state, snapshot, busy, installations } = deck;
const key = ref(''); const form = ref<AppSettings | null>(null); const saved = ref(false);
watch(() => state.value?.settings, settings => { if (settings && !form.value) form.value = JSON.parse(JSON.stringify(settings)); }, { immediate: true });
const totalBudget = computed({ get: () => form.value?.policy.totalBudgetUsd ?? '', set: value => { if (form.value) form.value.policy.totalBudgetUsd = value === '' ? null : Number(value); } });
const lifetime = computed({ get: () => form.value?.policy.maxLifetimeMinutes ?? '', set: value => { if (form.value) form.value.policy.maxLifetimeMinutes = value === '' ? null : Number(value); } });
const idleDelay = computed({ get: () => form.value?.policy.autoTerminateDelaySeconds ?? '', set: value => { if (form.value) form.value.policy.autoTerminateDelaySeconds = value === '' ? null : Number(value); } });
async function save() {
  if (!form.value) return;
  saved.value = false;
  await deck.perform('Saving settings', async () => {
    const settings = JSON.parse(JSON.stringify(form.value)) as AppSettings;
    settings.huggingFaceSecret = settings.huggingFaceSecret?.trim() || null;
    settings.codexPath = settings.codexPath?.trim() || null;
    settings.claudePath = settings.claudePath?.trim() || null;
    await deck.call('save_settings', { settings }); saved.value = true;
  });
}
async function saveKey() {
  const value = key.value; key.value = '';
  await deck.perform('Saving key in Keychain', () => deck.call('save_runpod_key', { key: value }));
}
async function browse(kind: 'codex' | 'claude') {
  const path = await deck.perform('Selecting CLI', () => deck.call<string | null>('choose_executable'));
  if (path && form.value) { if (kind === 'codex') form.value.codexPath = path; else form.value.claudePath = path; }
}
</script>
<template>
  <div class="page-heading"><div><div class="eyebrow">WORKSPACE SETTINGS</div><h1>Ready when you are.</h1><p>Set up credentials and limits before your first cloud test.</p></div><button class="button ghost" :disabled="!!busy" @click="deck.perform('Exporting sanitized diagnostics', () => deck.call('export_diagnostics'))">Export diagnostics</button></div>
  <div class="settings-layout" v-if="form">
    <section class="panel"><div class="panel-heading"><div><h2>Runpod account</h2><p>Your management key stays in the operating system credential store.</p></div><span :class="['tag', snapshot?.runpodKeyConfigured ? 'green' : '']">{{ snapshot?.credentialStoreError ? 'KEYCHAIN UNAVAILABLE' : snapshot?.runpodKeyConfigured ? 'KEY SAVED' : 'NOT CONNECTED' }}</span></div><form @submit.prevent="saveKey"><label>Runpod API key<input v-model="key" type="password" placeholder="Enter your API key" autocomplete="off" spellcheck="false" /></label><div class="button-row section-gap"><button type="submit" class="button ghost" :disabled="!!busy || !key">Save in Keychain</button><button v-if="snapshot?.runpodKeyConfigured" type="button" class="text-button" :disabled="!!busy" @click="deck.perform('Removing Runpod key', () => deck.call('remove_runpod_key'))">Remove saved key</button><a @click="deck.openExternal" href="https://console.runpod.io/user/settings" target="_blank" rel="noreferrer" class="push-right small">Runpod account ↗</a></div></form></section>
    <form @submit.prevent="save" class="settings-form">
      <section class="panel"><div class="panel-heading"><div><h2>Spending and cleanup</h2><p>GPU estimates exclude retained storage. Saved limits can shorten a running Pod’s deadline, but never extend it.</p></div></div><div class="notice warning">Independent expiry is not verified. If this app closes, sleeps, or loses connection, a Pod can continue billing beyond its deadline. Check Runpod to confirm cleanup.</div><div class="form-grid"><label>Maximum GPU rate (USD/hour)<input v-model.number="form.policy.maxHourlyUsd" type="number" min="0.01" :max="snapshot?.hourlyCeilingUsd" step="0.01" required /></label><label>Total experiment budget (USD)<input v-model="totalBudget" type="number" min="0.01" max="10000" step="0.01" placeholder="Required before provisioning" /></label><label>Maximum Pod lifetime (minutes)<input v-model="lifetime" type="number" min="1" max="1440" step="1" placeholder="Required before provisioning" /></label><label>Auto-finish after last session (seconds)<input v-model="idleDelay" type="number" min="30" max="3600" step="1" placeholder="Disabled — leave empty" /></label></div><label class="checkbox"><input v-model="form.policy.acknowledgeOfflineRisk" type="checkbox" /><span>I understand that cleanup cannot be guaranteed while the app is offline.</span></label><label class="checkbox"><input v-model="form.policy.paidProvisioningEnabled" type="checkbox" /><span>Enable paid provisioning when I confirm a deployment.</span></label></section>
      <section class="panel"><div class="panel-heading"><div><h2>Protected model access</h2><p>Only the Runpod secret name is stored here. Do not paste the Hugging Face token.</p></div></div><label>Hugging Face secret name<input v-model="form.huggingFaceSecret" placeholder="huggingface_token" autocomplete="off" spellcheck="false" /></label><p class="small muted">Accept the model’s access conditions on Hugging Face, create a read-scoped token, and store it in Runpod Secrets. The app checks access inside the Pod before downloading weights.</p></section>
      <section class="panel"><div class="panel-heading"><div><h2>Coding CLIs</h2><p>Existing installations only. The app does not install or upgrade your CLIs.</p></div><button type="button" class="button ghost compact" :disabled="!!busy" @click="deck.perform('Detecting CLIs', async () => { installations = await deck.call<CliInstallation[]>('detect_clis'); })">Detect</button></div><div class="cli-install" v-for="installation in installations" :key="installation.cli"><strong>{{ installation.cli === 'codex' ? 'Codex' : 'Claude Code' }} <span class="tag">{{ installation.version ?? 'NOT FOUND' }}</span></strong><p class="path">{{ installation.path ?? installation.message }}</p></div><label>Codex executable override<div class="input-action"><input v-model="form.codexPath" placeholder="Auto-detect" /><button type="button" class="button ghost" :disabled="!!busy" @click="browse('codex')">Browse</button></div></label><label>Claude Code executable override<div class="input-action"><input v-model="form.claudePath" placeholder="Auto-detect" /><button type="button" class="button ghost" :disabled="!!busy" @click="browse('claude')">Browse</button></div></label><label class="checkbox"><input v-model="form.policy.claudeEnabled" type="checkbox" /><span>Show experimental Claude Code integration.</span></label><p class="small muted">Launcher flags are checked for Codex 0.160.0 and Claude Code 2.1.289. Live model compatibility still needs your tests. Claude runs in bare mode to avoid subscription credentials and project provider overrides.</p></section>
      <div class="save-bar"><span class="muted small" role="status">{{ saved ? 'Settings saved.' : 'Window close keeps compute running. App quit asks how to handle it.' }}</span><button class="button primary" type="submit" :disabled="!!busy">Save settings</button></div>
    </form>
  </div>
</template>
