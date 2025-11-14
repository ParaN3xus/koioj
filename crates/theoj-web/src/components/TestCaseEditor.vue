<script setup lang="ts">
import { Icon } from "@iconify/vue";
import type { TestCaseData } from "@/theoj-api";

interface Props {
  testCase: TestCaseData & { fromZip?: boolean };
  index: number;
  title: string;
  removable?: boolean;
}

type Emits = (e: "remove") => void;

defineProps<Props>();
const emit = defineEmits<Emits>();

const handleRemove = () => {
  emit("remove");
};
</script>

<template>
  <div class="card bg-base-200">
    <div class="card-body p-4">
      <div class="flex items-center justify-between">
        <h4 class="font-semibold flex items-center gap-2">
          {{ title }}
          <span v-if="testCase.fromZip" class="badge badge-sm badge-info">
            From ZIP
          </span>
        </h4>
        <button v-if="removable" class="btn btn-sm btn-ghost btn-circle" @click="handleRemove">
          <Icon icon="fa7-solid:trash" width="14" />
        </button>
      </div>

      <!-- Manual Input -->
      <div v-if="!testCase.fromZip" class="grid grid-cols-2 gap-4">
        <div class="form-control">
          <label class="label label-text-alt">
            <span>Input</span>
            <span class="text-base-content/50">
              {{ testCase.input.length }} chars
            </span>
          </label>
          <textarea v-model="testCase.input" placeholder="Test input"
            class="textarea textarea-bordered textarea-sm font-mono leading-tight py-2" rows="4"></textarea>
        </div>
        <div class="form-control">
          <label class="label label-text-alt">
            <span>Output</span>
            <span class="text-base-content/50">
              {{ testCase.output.length }} chars
            </span>
          </label>
          <textarea v-model="testCase.output" placeholder="Expected output"
            class="textarea textarea-bordered textarea-sm font-mono leading-tight py-2" rows="4"></textarea>
        </div>
      </div>

      <!-- ZIP -->
      <div v-else class="flex gap-4 text-sm text-base-content/70">
        <span>Input: {{ testCase.input.length }} chars</span>
        <span>Output: {{ testCase.output.length }} chars</span>
      </div>
    </div>
  </div>
</template>
