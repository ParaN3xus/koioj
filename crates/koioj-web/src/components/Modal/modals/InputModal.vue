<script setup lang="ts">
import { ref } from 'vue';

interface Props {
  title?: string;
  placeholder?: string;
  initialValue?: string;
  confirmText?: string;
  cancelText?: string;
  inputType?: 'text' | 'password';
  errorMessage?: string;
  onConfirm?: (value: string) => void | Promise<void>;
  onCancel?: () => void;
}

const props = withDefaults(defineProps<Props>(), {
  title: '',
  placeholder: '',
  initialValue: '',
  confirmText: 'Confirm',
  cancelText: 'Cancel',
  inputType: 'text',
  errorMessage: '',
  onConfirm: undefined,
  onCancel: undefined
});

const emit = defineEmits<{
  close: [];
}>();

const inputValue = ref(props.initialValue);

const handleConfirm = async (): Promise<void> => {
  if (props.onConfirm) {
    await props.onConfirm(inputValue.value);
  }
  emit('close');
};

const handleCancel = (): void => {
  if (props.onCancel) {
    props.onCancel();
  }
  emit('close');
};

const handleBackdropClick = (e: Event): void => {
  e.preventDefault();
  handleCancel();
};
</script>

<template>
  <dialog class="modal modal-open">
    <div class="modal-box">
      <h3 class="text-lg font-bold">{{ title }}</h3>
      <div class="pt-4">
        <input v-model="inputValue" :type="inputType" :placeholder="placeholder" class="input input-bordered w-full"
          @keydown.enter="handleConfirm" autofocus />
        <div v-if="errorMessage" class="label">
          <span class="label-text-alt text-error">{{ errorMessage }}</span>
        </div>
      </div>
      <div class="modal-action">
        <div class="flex gap-2">
          <button class="btn btn-primary" @click="handleConfirm" type="button">
            {{ confirmText }}
          </button>
          <button class="btn btn-outline" @click="handleCancel" type="button">
            {{ cancelText }}
          </button>
        </div>
      </div>
    </div>
    <form method="dialog" class="modal-backdrop" @submit.prevent="handleBackdropClick">
      <button type="button">close</button>
    </form>
  </dialog>
</template>
