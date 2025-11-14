<script setup lang="ts">
import { marked } from "marked";
import markedKatex from "marked-katex-extension";
import { computed } from "vue";
import "katex/dist/katex.min.css";

interface Props {
  modelValue: string;
  placeholder?: string;
  rows?: number;
  showPreview?: boolean;
  required?: boolean;
}

const props = withDefaults(defineProps<Props>(), {
  placeholder: "",
  rows: 4,
  showPreview: false,
  required: false,
});

const emit = defineEmits<{
  "update:modelValue": [value: string];
}>();

const localValue = computed({
  get: () => props.modelValue,
  set: (value) => emit("update:modelValue", value),
});

const renderer = new marked.Renderer();
renderer.heading = ({ text, depth }) => {
  if (depth < 3) {
    depth = 3;
  }
  return `<h${depth}>${text}</h${depth}>`;
};
renderer.html = () => '';
marked.use(
  markedKatex({
    throwOnError: false,
  })
);
marked.use({
  breaks: true,
  gfm: true,
  renderer: renderer,
});

const renderMarkdown = (text: string) => {
  return marked(text);
};

const textareaClass = computed(() => {
  const baseClass = "textarea textarea-bordered w-full";
  const heightClass = `h-${props.rows * 6}`;
  return `${baseClass} ${heightClass}`;
});
</script>


<template>
  <div class="grid grid-cols-1 gap-4" :class="{ 'lg:grid-cols-2': showPreview }">
    <textarea v-model="localValue" :placeholder="placeholder" :class="textareaClass" :required="required"></textarea>
    <div v-if="showPreview" class="prose max-w-none p-4 border border-base-300 rounded-lg bg-base-200 overflow-auto"
      :class="textareaClass" v-html="renderMarkdown(modelValue)"></div>
  </div>
</template>
