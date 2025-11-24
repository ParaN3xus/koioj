<script setup lang="ts">
import { TestCaseJudgeResult } from "@/theoj-api";
import { formatResult } from "./utils.mts";

interface Props {
  result: TestCaseJudgeResult;
}

defineProps<Props>();

const testCaseResultColors: Record<TestCaseJudgeResult, string> = {
  [TestCaseJudgeResult.PENDING]: "badge-warning",
  [TestCaseJudgeResult.COMPILING]: "badge-info",
  [TestCaseJudgeResult.RUNNING]: "badge-info",
  [TestCaseJudgeResult.ACCEPTED]: "badge-success",
  [TestCaseJudgeResult.WRONG_ANSWER]: "badge-error",
  [TestCaseJudgeResult.TIME_LIMIT_EXCEEDED]: "badge-error",
  [TestCaseJudgeResult.MEMORY_LIMIT_EXCEEDED]: "badge-error",
  [TestCaseJudgeResult.RUNTIME_ERROR]: "badge-error",
  [TestCaseJudgeResult.COMPILE_ERROR]: "badge-error",
  [TestCaseJudgeResult.UNKNOWN_ERROR]: "badge-error",
};

</script>

<template>
  <div class="badge" :class="testCaseResultColors[result]">
    {{ formatResult(result) }}
  </div>
</template>
