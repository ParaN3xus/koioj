<script setup lang="ts">
import { SubmissionResult } from "@/koioj-api";
import { formatResult } from "./utils.mts";

interface Props {
  result: SubmissionResult;
}

defineProps<Props>();

const submissionResultColors: Record<SubmissionResult, string> = {
  [SubmissionResult.PENDING]: "badge-warning",
  [SubmissionResult.ACCEPTED]: "badge-success",
  [SubmissionResult.WRONG_ANSWER]: "badge-error",
  [SubmissionResult.TIME_LIMIT_EXCEEDED]: "badge-error",
  [SubmissionResult.MEMORY_LIMIT_EXCEEDED]: "badge-error",
  [SubmissionResult.RUNTIME_ERROR]: "badge-error",
  [SubmissionResult.COMPILE_ERROR]: "badge-error",
  [SubmissionResult.UNKNOWN_ERROR]: "badge-error",
};


</script>

<template>
  <div class="badge" :class="submissionResultColors[result]">
    {{ formatResult(result) }}
  </div>
</template>
