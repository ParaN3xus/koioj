<script setup lang="ts">
import hljs from "highlight.js";
import { computed, ref } from "vue";
import "highlight.js/styles/github-dark.css";
import { Icon } from "@iconify/vue";
import { useToast } from "vue-toastification";

interface Props {
  code: string;
  language?: string;
}

const props = defineProps<Props>();
const toast = useToast();
const copied = ref(false);

const highlightedCode = computed(() => {
  if (!props.language) {
    return props.code;
  }
  try {
    return hljs.highlight(props.code, { language: props.language }).value;
  } catch (e) {
    return props.code;
  }
});

const copyCode = async () => {
  try {
    await navigator.clipboard.writeText(props.code);
    copied.value = true;
    toast.success("Code copied to clipboard");
    setTimeout(() => {
      copied.value = false;
    }, 2000);
  } catch (err) {
    toast.error("Failed to copy code");
  }
};
</script>

<template>
  <div class="relative">
    <div class="absolute top-2 right-2 z-10">
      <button @click="copyCode" class="btn btn-sm btn-ghost" :class="{ 'btn-success': copied }">
        <Icon :icon="copied ? 'fa6-solid:check' : 'fa6-solid:copy'" class="w-4 h-4" />
      </button>
    </div>
    <pre class="bg-base-200 rounded-lg p-4 overflow-x-auto text-sm"><code
        :class="language ? `language-${language}` : ''"
        v-html="highlightedCode"
      ></code></pre>
  </div>
</template>
