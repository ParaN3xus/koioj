<script setup lang="ts">
import { Icon } from "@iconify/vue";
import { computed, onMounted, ref } from "vue";
import { useRoute, useRouter } from "vue-router";
import { useToast } from "vue-toastification";
import { useApiErrorHandler } from "@/composables/useApiErrorHandler.mjs";
import { buildPath, routeMap } from "@/routes.mjs";
import { useContestPasswordStore } from "@/stores/contestPassword.mjs";
import {
  type CancelablePromise,
  ContestService,
  type GetContestResponse,
  type GetProblemResponse,
  ProblemService,
  type SubmitRequest,
} from "@/theoj-api";

const route = useRoute();
const router = useRouter();
const toast = useToast();
const { handleApiError } = useApiErrorHandler();
const contestPasswordStore = useContestPasswordStore();

const problemId = computed(() => {
  // Contest mode: /contest/:contestId/problem/:problemId/submit
  if (route.params.problemId) {
    return route.params.problemId as string;
  }
  // Normal mode: /problem/:id/submit
  return route.params.id as string;
});

const contestId = computed(() => route.params.contestId as string | undefined);
const isContestMode = computed(() => !!contestId.value);

const code = ref<string>("");
const lang = ref<string>("cpp");
const isSubmitting = ref<boolean>(false);
const problemName = ref<string>("");
const contestData = ref<GetContestResponse | null>(null);

const languages = [
  { value: "cpp", label: "C++" },
  { value: "c", label: "C" },
  { value: "java", label: "Java" },
  { value: "python", label: "Python" },
];

onMounted(async () => {
  try {
    const problemResponse = await ProblemService.getProblem(problemId.value);
    problemName.value = problemResponse.name;

    // If in contest mode, also fetch contest data
    if (isContestMode.value && contestId.value) {
      const storedPassword = contestPasswordStore.getPassword(
        Number(contestId.value),
      );
      const contestResponse = await ContestService.getContest(
        contestId.value,
        storedPassword || null
      );
      contestData.value = contestResponse;
      document.title = `Submitting to ${problemResponse.name} in ${contestResponse.name} - TheOJ`;
    } else {
      document.title = `Submitting to ${problemResponse.name} - TheOJ`;
    }
  } catch (e) {
    handleApiError(e);
  }
});


const handleSubmit = async () => {
  if (!code.value.trim()) {
    toast.error("Code cannot be empty");
    return;
  }

  isSubmitting.value = true;

  try {
    const requestBody: SubmitRequest = {
      code: code.value,
      lang: lang.value,
      contestId: contestId.value || null,
    };

    const response = await ProblemService.submit(problemId.value, requestBody);

    toast.success("Submission created successfully");

    if (isContestMode.value) {
      if (!contestId.value) {
        toast.error("invalid contest!")
        return;
      }
      router.push(
        buildPath(routeMap.contestSubmission.path, {
          contestId: contestId.value,
          problemId: problemId.value,
          submissionId: response.submissionId,
        }),
      );
      return;
    }
    router.push(
      buildPath(routeMap.submission.path, {
        problemId: problemId.value,
        submissionId: response.submissionId,
      }),
    );
  } catch (error) {
    handleApiError(error);
  } finally {
    isSubmitting.value = false;
  }
};
</script>


<template>
  <div class="container mx-auto max-w-6xl">
    <div class="card bg-base-100 shadow-xl">
      <div class="card-body">
        <div class="mb-4">
          <h2 class="text-2xl font-bold flex flex-wrap items-center gap-2">
            <Icon icon="fa6-solid:code" class="w-6 h-6" />
            <span>Submit Solution</span>
            <template v-if="problemName">
              <span>to</span>
              <RouterLink :to="isContestMode
                ? buildPath(routeMap.contestProblem.path, { contestId: contestId!, problemId: problemId })
                : buildPath(routeMap.problem.path, { id: problemId })" class="link link-primary">
                {{ problemName }}
              </RouterLink>
            </template>
            <template v-if="isContestMode && contestData">
              <span>in</span>
              <RouterLink :to="buildPath(routeMap.contest.path, { id: contestId! })" class="link link-primary">
                {{ contestData.name }}
              </RouterLink>
            </template>
          </h2>
        </div>



        <div class="form-control w-full mb-4">
          <label class="label">
            <span class="label-text font-semibold">Programming Language</span>
          </label>
          <select v-model="lang" class="select select-bordered w-full">
            <option v-for="language in languages" :key="language.value" :value="language.value">
              {{ language.label }}
            </option>
          </select>
        </div>

        <div class="form-control w-full mb-4">
          <label class="label">
            <span class="label-text font-semibold">Code</span>
          </label>
          <textarea v-model="code" class="textarea textarea-bordered font-mono h-96"
            placeholder="Write your code here..."></textarea>
        </div>

        <div class="card-actions justify-end">
          <button @click="handleSubmit" class="btn btn-primary" :disabled="isSubmitting">
            <Icon v-if="isSubmitting" icon="fa6-solid:spinner" class="w-5 h-5 animate-spin" />
            <Icon v-else icon="fa6-solid:paper-plane" class="w-5 h-5" />
            {{ isSubmitting ? "Submitting..." : "Submit" }}
          </button>
        </div>
      </div>
    </div>
  </div>
</template>
