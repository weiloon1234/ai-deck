import { computed, ref } from 'vue';
import { invoke, isTauri } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type { AppError, AppSnapshot, CliInstallation, Hardware } from './contracts';
import defaults from './defaults.json';
import catalog from '../../model-catalog/catalog.json';

const snapshot = ref<AppSnapshot | null>(null);
const error = ref<AppError | null>(null);
const busy = ref<string | null>(null);
const modelImportBusy = ref<string | null>(null);
const modelImportNotice = ref('');
const installations = ref<CliInstallation[]>([]);
const hardware = ref<Hardware | null>(null);
const view = ref<'deployments' | 'models' | 'projects' | 'sessions' | 'setup'>('deployments');
const activeSessionId = ref<string | null>(null);
const selectedProfileId = ref(catalog.profiles[0]?.id ?? '');
const quitRequested = ref(false);
const clock = ref(Date.now() / 1000);
let refreshPromise: Promise<void> | null = null;
let refreshAgain = false;
const native = isTauri();

function asError(value: unknown): AppError {
  if (value && typeof value === 'object' && 'message' in value) {
    const e = value as Partial<AppError>;
    return { code: e.code ?? 'operation_failed', message: String(e.message), recovery: e.recovery ?? 'Try the operation again.' };
  }
  return { code: 'operation_failed', message: String(value), recovery: 'Check the current state and try again.' };
}

export function useDeck() {
  async function call<T>(command: string, args?: Record<string, unknown>): Promise<T> {
    if (!native) throw { code: 'desktop_required', message: 'Open the desktop app to use this control.', recovery: 'This browser view is a preview. It cannot create Pods or launch processes.' };
    return invoke<T>(command, args);
  }
  async function refresh(): Promise<void> {
    if (!native) return;
    if (refreshPromise) { refreshAgain = true; return refreshPromise; }
    refreshPromise = (async () => {
      do { refreshAgain = false; snapshot.value = await call<AppSnapshot>('snapshot'); } while (refreshAgain);
    })();
    try { await refreshPromise; } finally { refreshPromise = null; }
  }
  async function perform<T>(label: string, action: () => Promise<T>): Promise<T | undefined> {
    if (busy.value) return;
    busy.value = label; error.value = null;
    try { const result = await action(); await refresh(); return result; }
    catch (cause) { error.value = asError(cause); }
    finally { busy.value = null; }
  }
  async function modelOperation<T>(label: string, action: () => Promise<T>): Promise<T | undefined> {
    if (modelImportBusy.value) return;
    modelImportBusy.value = label; modelImportNotice.value = ''; error.value = null;
    try { const result = await action(); await refresh(); return result; }
    catch (cause) { error.value = asError(cause); }
    finally { modelImportBusy.value = null; }
  }
  async function initialize(): Promise<() => void> {
    const timer = window.setInterval(() => { clock.value = Date.now() / 1000; }, 1000);
    const listeners: UnlistenFn[] = [];
    if (native) {
      listeners.push(await listen('state-changed', () => { void refresh().catch(cause => { error.value = asError(cause); }); }));
      listeners.push(await listen('quit-requested', () => { quitRequested.value = true; }));
      listeners.push(await listen<{ message: string }>('model-import-progress', event => { if (modelImportBusy.value) modelImportBusy.value = event.payload.message; }));
      await perform('Loading your workspace', async () => { await refresh(); installations.value = await call<CliInstallation[]>('detect_clis'); });
    } else {
      snapshot.value = {
        state: { schemaVersion: 1, installationId: 'browser-preview', settings: defaults, projects: [], deployments: [], sessions: [], importedModels: [], enabledDeploymentId: null },
        catalog: catalog.profiles.map(model => ({ model, runtime: catalog.runtimes.find(r => r.id === model.runtimeId)!, fingerprint: 'Available in desktop diagnostics' })),
        runpodKeyConfigured: false, catalogError: null, credentialStoreError: null, hourlyCeilingUsd: defaults.policy.maxHourlyUsd, offlineExpiryVerified: false, appVersion: '0.1.0',
      } as AppSnapshot;
    }
    return () => { clearInterval(timer); listeners.forEach(unlisten => unlisten()); };
  }
  const state = computed(() => snapshot.value?.state);
  const activeDeployment = computed(() => state.value?.deployments.find(d => d.stage !== 'terminated'));
  const enabledDeployment = computed(() => state.value?.deployments.find(d => d.id === state.value?.enabledDeploymentId));
  const connected = computed(() => enabledDeployment.value?.stage === 'ready' && !!enabledDeployment.value.verifiedAt && clock.value - enabledDeployment.value.verifiedAt < 90);
  const activeSession = computed(() => state.value?.sessions.find(s => s.id === activeSessionId.value));
  const profile = computed(() => snapshot.value?.catalog.find(p => p.model.id === selectedProfileId.value));
  const runningSessions = computed(() => state.value?.sessions.filter(s => s.status !== 'ended') ?? []);
  function selectSession(id: string) { activeSessionId.value = id; view.value = 'sessions'; }
  function openExternal(event: MouseEvent) {
    if (!native) return;
    event.preventDefault();
    const url = (event.currentTarget as HTMLAnchorElement).href;
    void call('open_external', { url }).catch(report);
  }
  function report(cause: unknown) { error.value = asError(cause); }
  return { snapshot, state, error, busy, native, installations, hardware, view, activeSessionId, activeSession,
    activeDeployment, enabledDeployment, connected, profile, selectedProfileId, runningSessions, quitRequested, clock,
    modelImportBusy, modelImportNotice, modelOperation,
    call, refresh, perform, initialize, selectSession, report, openExternal };
}

export function money(value: number): string { return new Intl.NumberFormat('en-US', { style: 'currency', currency: 'USD', minimumFractionDigits: 2, maximumFractionDigits: 2 }).format(value); }
export function elapsed(seconds: number): string { const s = Math.max(0, Math.floor(seconds)); return `${Math.floor(s / 3600)}h ${Math.floor(s % 3600 / 60)}m ${s % 60}s`; }
export function stageLabel(value: string): string { return value.replace(/([a-z])([A-Z])/g, '$1 $2').replace(/^./, c => c.toUpperCase()); }
