<script setup lang="ts">
import { Icon } from "@iconify/vue";
import { onMounted, ref } from "vue";
import { useRoute, useRouter } from "vue-router";
import { useToast } from "vue-toastification";
import { useApiErrorHandler } from "@/composables/useApiErrorHandler.mjs";
import { buildPath, routeMap } from "@/routes.mjs";
import { ProblemService, type SubmitRequest } from "@/theoj-api";

const route = useRoute();
const router = useRouter();
const toast = useToast();
const { handleApiError } = useApiErrorHandler();

const problemId = ref<string>(route.params.id as string);
const code = ref<string>("");
const lang = ref<string>("cpp");
const contestId = ref<string | null>(null);
const isSubmitting = ref<boolean>(false);
const problemName = ref<string>("");

const languages = [
  { value: "cpp", label: "C++" },
  { value: "c", label: "C" },
  { value: "java", label: "Java" },
  { value: "python", label: "Python" },
];

onMounted(async () => {
  // Get contestId from query if exists
  if (route.query.contestId) {
    contestId.value = route.query.contestId as string;
  }
  try {
    const problemResponse = await ProblemService.getProblem(problemId.value);
    problemName.value = problemResponse.name;
    document.title = `Submitting to ${problemResponse.name} - TheOJ`;
  } catch (e) {
    handleApiError(e)
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
      contestId: contestId.value,
    };

    const response = await ProblemService.submit(problemId.value, requestBody);

    toast.success("Submission created successfully");

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
        <h2 class="card-title text-2xl mb-4">
          <Icon icon="fa6-solid:code" class="w-6 h-6" />
          Submit Solution
          <div v-if="problemName">
            <span>to </span>
            <RouterLink :to="buildPath(routeMap.problem.path, { id: problemId })" class="link link-primary">
              {{ problemName }}
            </RouterLink>
          </div>
        </h2>

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
