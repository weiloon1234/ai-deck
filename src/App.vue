<script setup lang="ts">
import { onMounted, onUnmounted, ref } from 'vue';
import { useDeck } from './shared/useDeck';
import DeploymentsView from './features/deployments/DeploymentsView.vue';
import ModelsView from './features/models/ModelsView.vue';
import ProjectsView from './features/projects/ProjectsView.vue';
import SessionsView from './features/sessions/SessionsView.vue';
import SetupView from './features/setup/SetupView.vue';
import LaunchSessionDialog from './features/sessions/LaunchSessionDialog.vue';
import ModalDialog from './shared/ModalDialog.vue';

const deck = useDeck();
const { state, view, busy, error, connected, enabledDeployment, runningSessions, quitRequested } = deck;
const launchOpen = ref(false);
const launchProjectId = ref<string | null>(null);
function openSession(projectId?: string) { launchProjectId.value = projectId ?? null; launchOpen.value = true; }
let cleanup: (() => void) | undefined;
onMounted(async () => { try { cleanup = await deck.initialize(); } catch (cause) { deck.report(cause); } });
onUnmounted(() => cleanup?.());
const navigation = [{ id: 'deployments', label: 'Deployment', icon: '◈' }, { id: 'models', label: 'Models', icon: '▦' }, { id: 'projects', label: 'Projects', icon: '⌑' }, { id: 'sessions', label: 'Sessions', icon: '⌘' }, { id: 'setup', label: 'Setup', icon: '⚙' }] as const;
async function quit(finishPods: boolean, acknowledgeRunningCosts = false) {
  await deck.perform('Closing AI Deck', () => deck.call('request_quit', { finishPods, acknowledgeRunningCosts }));
}
</script>
<template>
  <div class="app-shell">
    <aside class="sidebar">
      <div class="brand"><span class="brand-mark">A<span>·</span></span><div>AI <strong>Deck</strong><small>YOUR LOCAL CODING WORKSPACE</small></div></div>
      <nav aria-label="Main navigation"><button v-for="item in navigation" :key="item.id" :class="['nav-item', { selected: view === item.id }]" @click="view = item.id"><span aria-hidden="true">{{ item.icon }}</span>{{ item.label }}<span v-if="item.id === 'sessions' && runningSessions.length" class="count">{{ runningSessions.length }}</span></button></nav>
      <div class="sidebar-section"><span>LOCAL SESSIONS</span><button class="icon-button" aria-label="New coding session" @click="openSession()">+</button></div>
      <div class="sidebar-sessions">
        <button v-for="session in state?.sessions.slice().reverse()" :key="session.id" :class="['session-link', { selected: session.id === deck.activeSessionId.value && view === 'sessions' }]" @click="deck.selectSession(session.id)"><span :class="['dot', session.status]" /><div>{{ session.name }}<small>{{ session.cli === 'codex' ? 'Codex' : 'Claude Code' }} · {{ session.model }}</small></div></button>
        <p v-if="!state?.sessions.length" class="sidebar-empty">Your coding sessions<br />will appear here.</p>
      </div>
      <div class="sidebar-bottom"><div class="local-label"><span class="dot ready" />Files stay on this Mac</div><p>Prompts and selected code are sent to your hosted model.</p><button class="text-button" @click="quitRequested = true">Quit AI Deck ↗</button></div>
    </aside>
    <div class="workspace">
      <header class="topbar"><div class="breadcrumb">Workspace <span>/</span> {{ navigation.find(n => n.id === view)?.label }}</div><div class="connection"><span :class="['dot', connected ? 'ready' : '']" />{{ connected ? enabledDeployment?.profile.model.displayName : 'No enabled endpoint' }}<span class="tag">LOCAL EXECUTION</span></div></header>
      <div v-if="!deck.native" class="preview-banner">Browser preview · Cloud and terminal controls are available in the desktop app.</div>
      <div v-for="issue in [deck.snapshot.value?.catalogError, deck.snapshot.value?.credentialStoreError].filter(Boolean)" :key="issue!.code" class="error-banner" role="alert"><div><strong>{{ issue!.message }}</strong><p>{{ issue!.recovery }} Your saved deployments and cleanup controls remain available.</p></div><button class="button ghost" :disabled="!!busy" @click="deck.perform('Reloading workspace', deck.refresh)">Retry</button></div>
      <div v-if="error" class="error-banner" role="alert"><div><strong>{{ error.message }}</strong><p>{{ error.recovery }}</p></div><button class="icon-button" aria-label="Dismiss error" @click="error = null">×</button></div>
      <div v-if="busy" class="busy-line" role="status"><span class="spinner" />{{ busy }}…</div>
      <main>
        <DeploymentsView v-if="view === 'deployments'" @launch="openSession" />
        <ModelsView v-else-if="view === 'models'" />
        <ProjectsView v-else-if="view === 'projects'" @launch="openSession" />
        <SessionsView v-else-if="view === 'sessions'" @launch="openSession" />
        <SetupView v-else-if="view === 'setup'" />
      </main>
    </div>
    <LaunchSessionDialog v-if="launchOpen" :project-id="launchProjectId" @close="launchOpen = false" />
    <ModalDialog v-if="quitRequested" title="Finish your workspace?" :busy="!!busy" @close="quitRequested = false">
      <p class="dialog-copy">Closing local sessions does not terminate a Pod. Finish and Quit waits for Runpod to confirm deletion. Retained volumes keep their storage charges.</p>
      <div v-if="deck.activeDeployment.value" class="notice warning">An active or unresolved deployment may still be billing. Quitting also stops the app’s local cleanup timer.</div>
      <footer class="dialog-actions"><button class="button ghost" :disabled="!!busy" @click="quitRequested = false">Cancel</button><button v-if="deck.activeDeployment.value" class="button ghost" :disabled="!!busy" @click="quit(false, true)">Leave Pod running and quit</button><button class="button primary" :disabled="!!busy" @click="quit(true)">{{ deck.activeDeployment.value ? 'Finish and Quit' : 'Quit' }}</button></footer>
    </ModalDialog>
  </div>
</template>
