<script setup lang="ts">
import { onMounted, onUnmounted, ref, useId } from 'vue';
const props = defineProps<{ title: string; busy?: boolean }>();
const emit = defineEmits<{ close: [] }>();
const titleId = useId();
const dialog = ref<HTMLDialogElement>();
const previous = document.activeElement as HTMLElement | null;
onMounted(() => dialog.value?.showModal());
onUnmounted(() => previous?.focus());
function cancel(event: Event) { event.preventDefault(); if (!props.busy) emit('close'); }
</script>
<template>
  <dialog ref="dialog" class="dialog" @cancel="cancel" :aria-labelledby="titleId">
    <header class="dialog-header"><h2 :id="titleId">{{ title }}</h2><button class="icon-button" aria-label="Close dialog" :disabled="busy" @click="emit('close')">×</button></header>
    <slot />
  </dialog>
</template>
