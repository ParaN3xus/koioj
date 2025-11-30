<script setup lang="ts">
import { Icon } from "@iconify/vue";
import { computed, onMounted, onUnmounted, ref } from "vue";
import { RouterLink, useRoute } from "vue-router";
import SubmissionResultBadge from "@/components/Badges/SubmissionResultBadge.vue";
import TestCaseResultBadge from "@/components/Badges/TestCaseResultBadge.vue";
import CodeBlock from "@/components/CodeBlock.vue";
import { useApiErrorHandler } from "@/composables/useApiErrorHandler.mjs";
import {
  ContestService,
  type GetContestResponse,
  type GetSubmissionResponse,
  ProblemService,
  SubmissionResult,
} from "@/koioj-api";
import { buildPath, routeMap } from "@/routes.mjs";
import { useContestPasswordStore } from "@/stores/contestPassword.mjs";
import { APP_NAME, parseIntOrNull } from "@/utils.mjs";

const route = useRoute();
const { handleApiError } = useApiErrorHandler();
const contestPasswordStore = useContestPasswordStore();

const problemId = computed(() => {
  // Contest mode: /contest/:contestId/problem/:problemId/submission/:submissionId
  if (route.params.problemId) {
    return parseIntOrNull(route.params.problemId) ?? -1;
  }
  // Normal mode: /problem/:problemId/submission/:submissionId
  return parseIntOrNull(route.params.problemId) ?? -1;
});

const submissionId = ref<number>(
  parseIntOrNull(route.params.submissionId) ?? -1,
);
const contestId = computed(() => parseIntOrNull(route.params.contestId));
const isContestMode = computed(() => !!contestId.value);

const submission = ref<GetSubmissionResponse | null>(null);
const contestData = ref<GetContestResponse | null>(null);
const isLoading = ref<boolean>(true);
const pollingTimer = ref<number | null>(null);

const fetchSubmission = async () => {
  try {
    const submissionResponse = await ProblemService.getSubmission(
      problemId.value,
      submissionId.value,
    );
    submission.value = submissionResponse;

    // If in contest mode and haven't fetched contest data yet, fetch it
    if (isContestMode.value && contestId.value && !contestData.value) {
      const storedPassword = contestPasswordStore.getPassword(contestId.value);
      const contestResponse = await ContestService.getContest(
        contestId.value,
        storedPassword || null,
      );
      contestData.value = contestResponse;
      document.title = `Submission of ${submissionResponse.problemName} in ${contestResponse.name} - ${APP_NAME}`;
    } else {
      document.title = `Submission of ${submissionResponse.problemName} - ${APP_NAME}`;
    }

    isLoading.value = false;

    // pending -> poll again 3s later
    if (submissionResponse.result === SubmissionResult.PENDING) {
      pollingTimer.value = window.setTimeout(fetchSubmission, 3000);
    }
  } catch (error) {
    handleApiError(error);
    isLoading.value = false;
  }
};

onMounted(() => {
  fetchSubmission();
});

onUnmounted(() => {
  if (pollingTimer.value) {
    clearTimeout(pollingTimer.value);
  }
});
</script>


<template>
  <div class="container mx-auto max-w-6xl">
    <div class="card bg-base-100 shadow-xl">
      <div class="card-body">
        <h2 class="card-title text-2xl mb-4">
          <Icon icon="fa6-solid:file-code" class="w-6 h-6" />
          Submission Details
        </h2>

        <div v-if="isLoading" class="flex justify-center items-center py-12">
          <Icon icon="fa6-solid:spinner" class="w-8 h-8 animate-spin" />
        </div>

        <div v-else-if="submission" class="space-y-6">
          <!-- Submission Info -->
          <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
            <div>
              <p class="text-sm text-gray-500">Problem</p>
              <RouterLink :to="isContestMode
                ? buildPath(routeMap.contestProblem.path, { contestId: contestId!, problemId: problemId })
                : buildPath(routeMap.problem.path, { id: problemId })" class="font-semibold link">
                {{ submission.problemName }}
              </RouterLink>
            </div>
            <div v-if="isContestMode && contestData">
              <p class="text-sm text-gray-500">Contest</p>
              <RouterLink :to="buildPath(routeMap.contest.path, { id: contestId! })" class="font-semibold link">
                {{ contestData.name }}
              </RouterLink>
            </div>
            <div>
              <p class="text-sm text-gray-500">User</p>
              <p class="font-semibold">{{ submission.username }}</p>
            </div>
            <div>
              <p class="text-sm text-gray-500">Language</p>
              <p class="font-semibold uppercase">{{ submission.lang }}</p>
            </div>
            <div>
              <p class="text-sm text-gray-500">Submitted At</p>
              <p class="font-semibold">
                {{ new Date(submission.createdAt).toLocaleString() }}
              </p>
            </div>
            <div>
              <p class="text-sm text-gray-500">Result</p>
              <SubmissionResultBadge :result="submission.result" />
            </div>
            <div v-if="submission.timeConsumption !== null">
              <p class="text-sm text-gray-500">Time</p>
              <p class="font-semibold">{{ submission.timeConsumption }} ms</p>
            </div>
            <div v-if="submission.memConsumption !== null">
              <p class="text-sm text-gray-500">Memory</p>
              <p class="font-semibold">{{ submission.memConsumption }} KB</p>
            </div>
          </div>

          <!-- Test Cases -->
          <div>
            <h3 class="text-lg font-semibold mb-3">Test Cases</h3>
            <div class="overflow-x-auto">
              <table class="table table-zebra w-full">
                <thead>
                  <tr>
                    <th>Test Case ID</th>
                    <th>Result</th>
                  </tr>
                </thead>
                <tbody>
                  <tr v-for="testCase in submission.testCaseResults" :key="testCase.testCaseId">
                    <td class="font-mono">{{ testCase.testCaseId }}</td>
                    <td>
                      <TestCaseResultBadge :result="testCase.result" />
                    </td>
                  </tr>
                </tbody>
              </table>
            </div>
          </div>

          <!-- Code -->
          <div>
            <h3 class="text-lg font-semibold mb-3">Code</h3>
            <CodeBlock :code="submission.code" :language="submission.lang" />
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
