<script setup lang="ts">
import { Icon } from "@iconify/vue";
import { computed, onMounted, ref } from "vue";
import { useRoute, useRouter } from "vue-router";
import { useApiErrorHandler } from "@/composables/useApiErrorHandler.mjs";
import { useMarkdownRenderer } from "@/composables/useMarkdownRenderer.mts";
import type { GetProblemResponse, GetSolutionResponse } from "@/koioj-api";
import { ProblemService } from "@/koioj-api";
import { buildPath, routeMap } from "@/routes.mjs";
import { formatDateTime, parseIntOrNull } from "@/utils.mjs";

const route = useRoute();
const router = useRouter();
const { renderMarkdown } = useMarkdownRenderer();
const { handleApiError } = useApiErrorHandler();

const problemId = parseIntOrNull(route.params.problemId) ?? -1;
const solutionId = parseIntOrNull(route.params.solutionId) ?? -1;

const solution = ref<GetSolutionResponse | null>(null);
const problem = ref<GetProblemResponse | null>(null);
const loading = ref(true);

const renderedContent = computed(() => {
  if (!solution.value) return "";
  return renderMarkdown(solution.value.content);
});

const fetchData = async () => {
  try {
    loading.value = true;
    const [solutionData, problemData] = await Promise.all([
      ProblemService.getSolution(problemId, solutionId),
      ProblemService.getProblem(problemId),
    ]);
    solution.value = solutionData;
    problem.value = problemData;
  } catch (e) {
    handleApiError(e);
  } finally {
    loading.value = false;
  }
};

const goBack = () => {
  router.push(buildPath(routeMap.solution.path, { id: problemId }));
};

onMounted(() => {
  fetchData();
});
</script>

<template>
  <div class="container mx-auto max-w-6xl p-4">
    <div class="card bg-base-100 shadow-xl">
      <div class="card-body">
        <!-- Loading State -->
        <div v-if="loading" class="flex justify-center py-12">
          <span class="loading loading-spinner loading-lg"></span>
        </div>

        <!-- Content -->
        <template v-else-if="solution">
          <!-- Header -->
          <div>
            <h1 class="text-3xl font-bold mb-4">{{ solution.title }}</h1>

            <!-- Meta Information -->
            <div class="flex flex-wrap gap-4 text-sm text-base-content/70">
              <div class="flex items-center gap-2">
                <Icon icon="fa6-solid:user" class="w-4 h-4" />
                <span class="font-semibold">{{ solution.authorName }}</span>
              </div>
              <div class="flex items-center gap-2">
                <Icon icon="fa6-solid:calendar" class="w-4 h-4" />
                <span>{{ formatDateTime(solution.createdAt) }}</span>
              </div>
              <div class="flex items-center gap-2">
                <Icon icon="fa6-solid:hashtag" class="w-4 h-4" />
                <span class="font-mono text-xs">{{ solution.solutionId }}</span>
              </div>
            </div>
          </div>

          <div class="divider"></div>

          <!-- Solution Content -->
          <div class="prose max-w-none" v-html="renderedContent"></div>
        </template>

        <!-- Error State -->
        <div v-else class="text-center py-12">
          <Icon icon="fa6-solid:triangle-exclamation" class="w-16 h-16 mx-auto text-error mb-4" />
          <p class="text-lg font-semibold mb-2">Solution not found</p>
          <button @click="goBack" class="btn btn-primary">
            <Icon icon="fa6-solid:arrow-left" class="w-5 h-5" />
            Back to Solutions
          </button>
        </div>
      </div>
    </div>
  </div>
</template>
