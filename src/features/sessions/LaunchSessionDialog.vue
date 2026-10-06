<script setup lang="ts">
import { computed, ref } from 'vue';
import ModalDialog from '../../shared/ModalDialog.vue';
import { useDeck } from '../../shared/useDeck';
import type { CliKind } from '../../shared/contracts';
const props = defineProps<{ projectId?: string | null }>();
const emit = defineEmits<{ close: [] }>();
const deck = useDeck();
const { state, connected, enabledDeployment, busy } = deck;
const cli = ref<CliKind>('codex'); const projectId = ref(props.projectId ?? state.value?.projects[0]?.id ?? '');
const name = ref('Coding session'); const allowUnverified = ref(false);
const shared = computed(() => state.value?.sessions.filter(s => s.projectId === projectId.value && s.status !== 'ended') ?? []);
async function launch() {
  const id = await deck.perform('Opening isolated CLI', () => deck.call<string>('launch_session', { request: { projectId: projectId.value, cli: cli.value, name: name.value, allowUnverified: allowUnverified.value } }));
  if (id) { deck.selectSession(id); emit('close'); }
}
</script>
<template>
  <ModalDialog title="Open a coding session" :busy="!!busy" @close="emit('close')">
    <div v-if="!connected" class="notice warning">Enable a ready endpoint first. No connected CLI can open without it.</div>
    <form @submit.prevent="launch"><div class="cli-choices"><label :class="{ selected: cli === 'codex' }"><input v-model="cli" type="radio" value="codex" /><strong>Codex</strong><span>Responses API</span></label><label :class="{ selected: cli === 'claude' }"><input v-model="cli" type="radio" value="claude" :disabled="!state?.settings.policy.claudeEnabled" /><strong>Claude Code</strong><span>Experimental</span></label></div>
      <label>Local project<select v-model="projectId" required><option value="" disabled>Choose a project</option><option v-for="project in state?.projects" :key="project.id" :value="project.id">{{ project.name }} · {{ project.path }}</option></select></label>
      <button v-if="!state?.projects.length" type="button" class="text-button" :disabled="!!busy" @click="deck.perform('Adding project', async () => { const project = await deck.call<{ id: string } | null>('choose_project'); if (project) projectId = project.id; })">+ Add a local project</button>
      <label>Session name<input v-model="name" required maxlength="80" /></label>
      <div v-if="shared.length" class="notice warning">{{ shared.length }} session(s) already use this folder. They can edit the same files. Choose another folder if you need a separate workspace.</div>
      <div class="binding"><span>BOUND AT LAUNCH</span><strong>{{ enabledDeployment?.profile.model.displayName ?? 'No endpoint enabled' }}</strong><small>{{ enabledDeployment?.id ?? 'Provision → Enable → Open CLI' }}</small></div>
      <label class="checkbox"><input v-model="allowUnverified" type="checkbox" /><span>I am testing this candidate combination. Its coding compatibility has not been verified.</span></label>
      <p v-if="cli === 'claude'" class="small muted">Anthropic does not support Claude Code with non-Claude models. This path remains experimental even after successful tests.</p>
      <footer class="dialog-actions"><button type="button" class="button ghost" :disabled="!!busy" @click="emit('close')">Cancel</button><button type="submit" class="button primary" :disabled="!!busy || !connected || !projectId || !name.trim()">Open {{ cli === 'codex' ? 'Codex' : 'Claude Code' }} →</button></footer>
    </form>
  </ModalDialog>
</template>
