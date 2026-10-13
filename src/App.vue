<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from 'vue';
import { useDeck, stageLabel } from './shared/useDeck';
import UiIcon from './shared/UiIcon.vue';
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
const workspaceElement = ref<HTMLElement>();
const historyExpanded = ref(false);
const endedSessionCount = computed(() => state.value?.sessions.filter(session => session.status === 'ended').length ?? 0);
const selectedProvider = computed(() => deck.snapshot.value?.providers?.find(p => p.info.id === state.value?.settings.provider)?.info);
const sidebarSessions = computed(() => (state.value?.sessions ?? []).filter(session => historyExpanded.value || session.status !== 'ended').slice().reverse());
watch(view, () => workspaceElement.value?.scrollIntoView({ block: 'start' }), { flush: 'post' });
function openSession(projectId?: string) { launchProjectId.value = projectId ?? null; launchOpen.value = true; }
let cleanup: (() => void) | undefined;
onMounted(async () => { try { cleanup = await deck.initialize(); } catch (cause) { deck.report(cause); } });
onUnmounted(() => cleanup?.());
const navigation = [
  { id: 'deployments', label: 'Deployment' },
  { id: 'models', label: 'Models' },
  { id: 'projects', label: 'Projects' },
  { id: 'sessions', label: 'Sessions' },
  { id: 'setup', label: 'Setup' },
] as const;
async function quit(finishPods: boolean, acknowledgeRunningCosts = false) {
  await deck.perform('Closing AI Deck', () => deck.call('request_quit', { finishPods, acknowledgeRunningCosts }));
}
</script>
<template>
  <div class="app-shell">
    <a class="skip-link" href="#workspace-content">Skip to workspace</a>
    <aside class="sidebar">
      <div class="brand"><span class="brand-mark">A<span>·</span></span><div>AI <strong>Deck</strong><small>Local coding workspace</small></div></div>
      <nav aria-label="Main navigation"><button v-for="item in navigation" :key="item.id" :class="['nav-item', { selected: view === item.id }]" :aria-current="view === item.id ? 'page' : undefined" @click="view = item.id"><UiIcon :name="item.id" />{{ item.label }}<span v-if="item.id === 'sessions' && runningSessions.length" class="count">{{ runningSessions.length }}</span></button></nav>
      <div class="sidebar-section"><span>SESSIONS</span><button class="icon-button" aria-label="New coding session" @click="openSession()">+</button></div>
      <div class="sidebar-sessions">
        <button v-for="session in sidebarSessions" :key="session.id" :class="['session-link', { selected: session.id === deck.activeSessionId.value && view === 'sessions' }]" :aria-current="session.id === deck.activeSessionId.value && view === 'sessions' ? 'page' : undefined" :title="`${session.name} · ${stageLabel(session.status)}`" @click="deck.selectSession(session.id)"><span :class="['dot', session.status]" /><div>{{ session.name }}<small>{{ session.cli === 'codex' ? 'Codex' : 'Claude Code' }} · {{ session.model }}</small></div><span class="visually-hidden">{{ stageLabel(session.status) }}</span></button>
        <p v-if="!sidebarSessions.length" class="sidebar-empty">No open sessions yet.</p>
        <button v-if="endedSessionCount" class="session-history-toggle" :aria-expanded="historyExpanded" @click="historyExpanded = !historyExpanded">{{ historyExpanded ? 'Hide' : 'Show' }} ended sessions <span>{{ endedSessionCount }}</span></button>
      </div>
      <div class="sidebar-bottom"><div class="local-label"><span class="dot ready" />Files stay on this Mac</div><p>Prompts and selected code are sent to your hosted model.</p><button class="text-button" @click="quitRequested = true">Quit AI Deck ↗</button></div>
    </aside>
    <div ref="workspaceElement" class="workspace">
      <header class="topbar"><div class="breadcrumb">Workspace <span aria-hidden="true">/</span> <strong>{{ navigation.find(n => n.id === view)?.label }}</strong></div><button v-if="selectedProvider" class="text-button" @click="view = 'setup'">{{ selectedProvider.name }}{{ selectedProvider.provisioningAvailable ? '' : ' · setup only' }}</button><div class="connection"><span :class="['dot', connected ? 'ready' : '']" />{{ connected ? enabledDeployment?.profile.model.displayName : 'No enabled endpoint' }}</div></header>
      <div v-if="!deck.native" class="preview-banner">Browser preview · Cloud and terminal controls are available in the desktop app.</div>
      <div v-for="issue in [deck.snapshot.value?.catalogError, deck.snapshot.value?.credentialStoreError].filter(Boolean)" :key="issue!.code" class="error-banner" role="alert"><div><strong>{{ issue!.message }}</strong><p>{{ issue!.recovery }} Your saved deployments and cleanup controls remain available.</p></div><button class="button ghost" :disabled="!!busy" @click="deck.perform('Reloading workspace', deck.refresh)">Retry</button></div>
      <div v-if="error" class="error-banner" role="alert"><div><strong>{{ error.message }}</strong><p>{{ error.recovery }}</p></div><button class="icon-button" aria-label="Dismiss error" @click="error = null">×</button></div>
      <div v-if="busy" class="busy-line" role="status"><span class="spinner" />{{ busy }}…</div>
      <main id="workspace-content" tabindex="-1">
        <DeploymentsView v-if="view === 'deployments'" @launch="openSession" />
        <ProjectsView v-else-if="view === 'projects'" @launch="openSession" />
        <SessionsView v-else-if="view === 'sessions'" @launch="openSession" />
        <div v-show="view === 'models'"><ModelsView /></div>
        <div v-show="view === 'setup'"><SetupView /></div>
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
