<script setup lang="ts">
import { onMounted, onUnmounted, ref } from 'vue';
import { Terminal } from '@xterm/xterm';
import { FitAddon } from '@xterm/addon-fit';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import '@xterm/xterm/css/xterm.css';
import { useDeck } from '../../shared/useDeck';
import type { TerminalChunk, TerminalReplay } from '../../shared/contracts';
const props = defineProps<{ sessionId: string }>();
const container = ref<HTMLDivElement>(); const deck = useDeck();
let terminal: Terminal | undefined; let stop: UnlistenFn | undefined; let observer: ResizeObserver | undefined;
let disposed = false; let lastSequence = 0; let hydrated = false; const pending: TerminalChunk[] = [];
function write(chunk: TerminalChunk) { if (chunk.sequence > lastSequence) { terminal?.write(chunk.text); lastSequence = chunk.sequence; } }
onMounted(async () => {
  terminal = new Terminal({ cursorBlink: true, fontSize: 13, fontFamily: '"SFMono-Regular", Menlo, Consolas, monospace', scrollback: 6000,
    theme: { background: '#0b1118', foreground: '#dbe5ed', cursor: '#a4efbd', selectionBackground: '#354757' } });
  const fit = new FitAddon(); terminal.loadAddon(fit); terminal.open(container.value!); fit.fit();
  terminal.onData(data => { void deck.call('terminal_input', { id: props.sessionId, data }).catch(deck.report); });
  terminal.attachCustomKeyEventHandler(event => {
    if (event.type === 'keydown' && (event.metaKey || event.ctrlKey && event.shiftKey) && event.key.toLowerCase() === 'c' && terminal?.hasSelection()) {
      void navigator.clipboard.writeText(terminal.getSelection()).catch(deck.report); return false;
    }
    return true;
  });
  observer = new ResizeObserver(() => { if (disposed) return; fit.fit(); void deck.call('terminal_resize', { id: props.sessionId, rows: terminal!.rows, cols: terminal!.cols }).catch(() => {}); });
  observer.observe(container.value!);
  if (!deck.native) { terminal.writeln('Open AI Deck on your Mac to connect a terminal.'); return; }
  try {
    stop = await listen<TerminalChunk>('terminal-output', event => { if (event.payload.sessionId !== props.sessionId) return; if (!hydrated) pending.push(event.payload); else write(event.payload); });
    if (disposed) { stop(); return; }
    const replay = await deck.call<TerminalReplay>('terminal_replay', { id: props.sessionId });
    if (disposed) return;
    [...replay.chunks, ...pending].sort((a, b) => a.sequence - b.sequence).forEach(write); hydrated = true;
    if (!replay.chunks.length && !replay.running) terminal.writeln('\r\nThis process has ended. Open its CLI resume history to continue.\r\nTerminal scrollback is kept only while this app is running.');
    terminal.focus();
  } catch (error) { if (!disposed) deck.report(error); }
});
onUnmounted(() => { disposed = true; stop?.(); observer?.disconnect(); terminal?.dispose(); });
</script>
<template><div ref="container" class="terminal-container" aria-label="Interactive coding CLI terminal" /></template>
