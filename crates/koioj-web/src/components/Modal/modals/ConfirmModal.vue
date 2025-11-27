<script setup lang="ts">
interface Props {
  title?: string;
  reverseColors?: boolean;
  reverseOrder?: boolean;
  onYes?: () => void | Promise<void>;
  onNo?: () => void;
}

const props = withDefaults(defineProps<Props>(), {
  title: '',
  reverseColors: false,
  reverseOrder: false,
  onYes: undefined,
  onNo: undefined
});

const emit = defineEmits<{
  close: [];
}>();

const handleYes = async (): Promise<void> => {
  if (props.onYes) {
    await props.onYes();
  }
  emit('close');
};

const handleNo = (): void => {
  if (props.onNo) {
    props.onNo();
  }
  emit('close');
};

const handleBackdropClick = (e: Event): void => {
  e.preventDefault();
  handleNo();
};
</script>

<template>
  <dialog class="modal">
    <div class="modal-box">
      <h3 class="text-lg font-bold">{{ title }}</h3>
      <div class="py-4">
        <slot></slot>
      </div>
      <div class="modal-action">
        <div :class="['flex gap-2', reverseOrder ? 'flex-row-reverse' : 'flex-row']">
          <button :class="['btn w-16', reverseColors ? 'btn-outline' : 'btn-primary']" @click="handleYes" type="button">
            Yes
          </button>
          <button :class="['btn w-16', reverseColors ? 'btn-primary' : 'btn-outline']" @click="handleNo" type="button">
            No
          </button>
        </div>
      </div>
    </div>
    <form method="dialog" class="modal-backdrop" @submit.prevent="handleBackdropClick">
      <button>close</button>
    </form>
  </dialog>
</template>
