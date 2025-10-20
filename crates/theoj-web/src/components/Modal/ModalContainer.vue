<script setup lang="ts">
import { onMounted, onUnmounted, ref } from "vue";
import { modalService } from "./modalService.mts";
import type { Modal } from "./types.mts";

const modal = ref<Modal | null>(null);

const updateModal = (newModal: Modal | null): void => {
  modal.value = newModal;
};

const closeModal = (): void => {
  modalService.close();
};

onMounted(() => {
  modalService.subscribe(updateModal);
});

onUnmounted(() => {
  modalService.unsubscribe(updateModal);
});
</script>

<template>
  <div id="modal-root">
    <component v-if="modal" :is="modal.component" v-bind="modal.attrs" @close="closeModal">
      <template v-if="modal.slots?.default" #default>
        <div v-html="modal.slots.default"></div>
      </template>
    </component>
  </div>
</template>
