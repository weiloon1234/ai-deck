<script setup lang="ts">
import { computed, ref, watch } from 'vue';
import { useDeck } from '../../shared/useDeck';
import type { AppSettings, CliInstallation, ProviderId } from '../../shared/contracts';
const deck = useDeck();
const { state, snapshot, busy, installations } = deck;
const key = ref(''); const form = ref<AppSettings | null>(null); const saved = ref(false);
const keyNotice = ref(''); const connectionNotice = ref('');
const providerId = computed(() => state.value?.settings.provider ?? 'runpod');
const provider = computed(() => snapshot.value?.providers.find(p => p.info.id === providerId.value));
watch(() => state.value?.settings, settings => { if (settings && !form.value) form.value = JSON.parse(JSON.stringify(settings)); }, { immediate: true });
watch(providerId, id => {
  if (form.value) form.value.provider = id;
  key.value = ''; keyNotice.value = ''; connectionNotice.value = '';
}, { flush: 'sync' });
watch(key, () => { keyNotice.value = ''; connectionNotice.value = ''; }, { flush: 'sync' });
watch(() => provider.value?.keyConfigured, () => { connectionNotice.value = ''; }, { flush: 'sync' });
const dirty = computed(() => !!form.value && JSON.stringify(form.value) !== JSON.stringify(state.value?.settings));
watch(form, () => { saved.value = false; }, { deep: true, flush: 'sync' });
const totalBudget = computed({ get: () => form.value?.policy.totalBudgetUsd ?? '', set: value => { if (form.value) form.value.policy.totalBudgetUsd = value === '' ? null : Number(value); } });
const lifetime = computed({ get: () => form.value?.policy.maxLifetimeMinutes ?? '', set: value => { if (form.value) form.value.policy.maxLifetimeMinutes = value === '' ? null : Number(value); } });
const idleDelay = computed({ get: () => form.value?.policy.autoTerminateDelaySeconds ?? '', set: value => { if (form.value) form.value.policy.autoTerminateDelaySeconds = value === '' ? null : Number(value); } });
async function save() {
  if (!form.value) return;
  saved.value = false;
  await deck.perform('Saving settings', async () => {
    const draft = JSON.stringify(form.value);
    const settings = JSON.parse(draft) as AppSettings;
    settings.huggingFaceSecret = settings.huggingFaceSecret?.trim() || null;
    settings.codexPath = settings.codexPath?.trim() || null;
    settings.claudePath = settings.claudePath?.trim() || null;
    await deck.call('save_settings', { settings });
    if (JSON.stringify(form.value) === draft) { form.value = settings; saved.value = true; }
  });
}
async function saveKey() {
  if (busy.value) return;
  const provider = providerId.value;
  const value = key.value; key.value = '';
  const result = await deck.perform('Saving key in Keychain', async () => { await deck.call('save_provider_key', { provider, key: value }); return true; });
  if (result && provider === providerId.value) keyNotice.value = 'Key saved in Keychain. Check read access when you are ready.';
}
async function selectProvider(event: Event) {
  const select = event.target as HTMLSelectElement;
  await deck.perform('Selecting cloud provider', () => deck.call('select_provider', { provider: select.value as ProviderId }));
  select.value = providerId.value;
}
async function removeKey() {
  keyNotice.value = ''; connectionNotice.value = '';
  await deck.perform('Removing provider key', () => deck.call('remove_provider_key', { provider: providerId.value }));
}
async function checkConnection() {
  const id = providerId.value;
  connectionNotice.value = '';
  const result = await deck.perform('Checking provider read access', async () => { await deck.call('check_provider_connection', { provider: id }); return true; });
  if (result && id === providerId.value) connectionNotice.value = id === 'gputw'
    ? 'GPUtw accepted the key for instances:read. Provisioning is still unavailable; startup, recovery and other scopes were not tested.'
    : 'Runpod returned hardware and storage information. No Pod was created.';
}
async function browse(kind: 'codex' | 'claude') {
  const path = await deck.perform('Selecting CLI', () => deck.call<string | null>('choose_executable'));
  if (path && form.value) { if (kind === 'codex') form.value.codexPath = path; else form.value.claudePath = path; }
}
</script>
<template>
  <div class="page-heading"><div><h1>Setup</h1><p>Connect your account, set spending limits and choose your coding tools.</p></div><button class="button ghost" :disabled="!!busy" @click="deck.perform('Exporting sanitized diagnostics', () => deck.call('export_diagnostics'))">Export diagnostics</button></div>
  <div class="settings-layout" v-if="form">
    <section class="panel provider-setup" v-if="provider">
      <div class="panel-heading"><div><h2>Cloud provider</h2><p>Choose the provider for new deployments. Existing deployments and sessions keep their original provider.</p></div></div>
      <label for="cloud-provider">Provider<select id="cloud-provider" :value="providerId" :disabled="!!busy" @change="selectProvider"><option v-for="option in snapshot?.providers" :key="option.info.id" :value="option.info.id">{{ option.info.name }}{{ option.info.provisioningAvailable ? '' : ' · setup only' }}</option></select></label>
      <p class="small muted">Selection saves immediately. Spending limits and coding tools are shared across providers.</p>
      <div v-if="!provider.info.provisioningAvailable" class="notice warning provider-readiness"><strong>GPUtw setup is available. Provisioning is pending.</strong><p>You can save a key and check read access without renting a GPU. The runtime image is published.</p><details><summary>What still needs confirmation</summary><ul><li v-for="limit in provider.info.limitations" :key="limit">{{ limit }}</li></ul></details></div>
      <div class="panel-heading section-gap"><h3>{{ provider.info.name }} account</h3><span :class="['tag', provider.keyConfigured ? 'green' : '']">{{ provider.credentialStoreError ? 'KEYCHAIN UNAVAILABLE' : provider.keyConfigured ? 'KEY SAVED' : 'NO KEY SAVED' }}</span></div>
      <p class="small muted">Each provider has its own Keychain entry. Saving a key does not verify it or create compute.</p>
      <form @submit.prevent="saveKey">
        <label>{{ provider.info.name }} API key<input v-model="key" type="password" placeholder="Enter your API key" autocomplete="off" autocapitalize="off" spellcheck="false" :disabled="!!busy" /></label>
        <p v-if="providerId === 'gputw'" class="small muted">Use <code>instances:read</code> for this check. Deployment and management permissions are not tested.</p>
        <div class="button-row wrap section-gap"><button type="submit" class="button ghost" :disabled="!!busy || !key.trim()">Save in Keychain</button><button type="button" class="button ghost" :disabled="!!busy || !provider.keyConfigured" @click="checkConnection">Check read access</button><button v-if="provider.keyConfigured" type="button" class="text-button" :disabled="!!busy" @click="removeKey">Remove saved key</button></div>
      </form>
      <p v-if="keyNotice" class="small" role="status">{{ keyNotice }}</p><p v-if="connectionNotice" class="notice" role="status">{{ connectionNotice }}</p>
      <div class="button-row wrap section-gap"><a @click="deck.openExternal" :href="provider.info.accountUrl" target="_blank" rel="noreferrer" class="small">{{ provider.info.name }} account ↗</a><a v-if="provider.info.keyDocsUrl !== provider.info.accountUrl" @click="deck.openExternal" :href="provider.info.keyDocsUrl" target="_blank" rel="noreferrer" class="small">API key permissions ↗</a></div>
    </section>
    <form @submit.prevent="save" class="settings-form">
      <section class="panel"><div class="panel-heading"><div><h2>Spending and cleanup</h2><p>GPU estimates exclude retained storage. Saved limits can shorten a deployment’s deadline, but never extend it.</p></div></div><div class="notice warning">Independent expiry is not verified. If this app closes, sleeps, or loses connection, compute can continue billing beyond its deadline. Check the deployment’s provider to confirm cleanup.</div><div class="form-grid"><label>Maximum GPU rate (USD/hour)<input v-model.number="form.policy.maxHourlyUsd" type="number" min="0.01" :max="snapshot?.hourlyCeilingUsd" step="0.01" required /></label><label>Total experiment budget (USD)<input v-model="totalBudget" type="number" min="0.01" max="10000" step="0.01" placeholder="Required before provisioning" /></label><label>Maximum deployment lifetime (minutes)<input v-model="lifetime" type="number" min="1" max="1440" step="1" placeholder="Required before provisioning" /></label><label>Auto-finish after last session (seconds)<input v-model="idleDelay" type="number" min="30" max="3600" step="1" placeholder="Disabled — leave empty" /></label></div><label class="checkbox"><input v-model="form.policy.acknowledgeOfflineRisk" type="checkbox" /><span>I understand that cleanup cannot be guaranteed while the app is offline.</span></label><label class="checkbox"><input v-model="form.policy.paidProvisioningEnabled" type="checkbox" /><span>Enable paid provisioning for supported providers when I confirm a deployment.</span></label></section>
      <section class="panel"><div class="panel-heading"><div><h2>Protected model access · Runpod</h2><p>Only the Runpod secret name is stored here. Do not paste the Hugging Face token.</p></div></div><label>Hugging Face secret name<input v-model="form.huggingFaceSecret" placeholder="huggingface_token" autocomplete="off" spellcheck="false" /></label><p class="small muted">Accept the model’s access conditions on Hugging Face, create a read-scoped token, and store it in Runpod Secrets. The app checks access inside the Pod before downloading weights. This reference is used only by Runpod.</p></section>
      <section class="panel">
        <div class="panel-heading"><div><h2>Coding CLIs</h2><p>Existing installations only. The app does not install or upgrade your CLIs.</p></div><button type="button" class="button ghost compact" :disabled="!!busy" @click="deck.perform('Detecting CLIs', async () => { installations = await deck.call<CliInstallation[]>('detect_clis'); })">Detect</button></div>
        <div class="cli-install" v-for="installation in installations" :key="installation.cli"><strong>{{ installation.cli === 'codex' ? 'Codex' : 'Claude Code' }} <span class="tag">{{ installation.version ?? 'NOT FOUND' }}</span></strong><p class="path">{{ installation.path ?? installation.message }}</p></div>
        <div class="cli-override"><label for="codex-path">Codex executable override</label><div class="input-action"><input id="codex-path" v-model="form.codexPath" placeholder="Auto-detect" /><button type="button" class="button ghost" aria-label="Browse for Codex executable" :disabled="!!busy" @click="browse('codex')">Browse</button></div></div>
        <div class="cli-override"><label for="claude-path">Claude Code executable override</label><div class="input-action"><input id="claude-path" v-model="form.claudePath" placeholder="Auto-detect" /><button type="button" class="button ghost" aria-label="Browse for Claude Code executable" :disabled="!!busy" @click="browse('claude')">Browse</button></div></div>
        <label class="checkbox"><input v-model="form.policy.claudeEnabled" type="checkbox" /><span>Show experimental Claude Code integration.</span></label><p class="small muted">Launcher flags are checked for Codex 0.160.0 and Claude Code 2.1.289. Live model compatibility still needs your tests. Claude runs in bare mode to avoid subscription credentials and project provider overrides.</p>
      </section>
      <div class="save-bar"><span :class="['small', dirty ? 'amber-text' : 'muted']" role="status">{{ dirty ? 'Unsaved changes' : saved ? 'Settings saved.' : 'No unsaved changes' }}</span><button class="button primary" type="submit" :disabled="!!busy || !dirty">Save settings</button></div>
    </form>
  </div>
</template>
