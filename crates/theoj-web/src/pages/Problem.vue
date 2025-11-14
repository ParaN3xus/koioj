<script setup lang="ts">
import { Icon } from "@iconify/vue";
import { computed, onMounted, ref } from "vue";
import { useRoute, useRouter } from "vue-router";
import { useToast } from "vue-toastification";
import { useApiErrorHandler } from "@/composables/useApiErrorHandler.mjs";
import { useMarkdownRenderer } from "@/composables/useMarkdownRenderer.mts";
import { buildPath, routeMap } from "@/routes.mjs";
import {
  type GetProblemResponse,
  ProblemService,
  ProblemStatus,
  UserRole,
  UserService,
} from "@/theoj-api";
import { useUserStore } from "@/user.mjs";

const { handleApiError } = useApiErrorHandler();
const { renderMarkdown } = useMarkdownRenderer();
const router = useRouter();
const route = useRoute();
const toast = useToast();
const userStore = useUserStore();

const problemId = computed(() => route.params.id as string);
const isLoading = ref(true);
const currentUserRole = ref<UserRole | null>(null);
const problemData = ref<GetProblemResponse | null>(null);

const isAdminOrTeacher = computed(() => {
  return (
    currentUserRole.value === UserRole.ADMIN ||
    currentUserRole.value === UserRole.TEACHER
  );
});

const loadProblemData = async () => {
  isLoading.value = true;
  try {
    const [roleResponse, problemResponse] = await Promise.all([
      UserService.getRole(userStore.userId),
      ProblemService.getProblem(problemId.value),
    ]);

    currentUserRole.value = roleResponse.role;
    problemData.value = problemResponse;
  } catch (e) {
    handleApiError(e);
    router.push(routeMap.index.path);
  } finally {
    isLoading.value = false;
  }
};

onMounted(() => {
  loadProblemData();
});

const handleEdit = () => {
  router.push(buildPath(routeMap.editProblem.path, { id: problemId.value }));
};

const handleSubmit = () => {
  // router.push(buildPath(routeMap.problem.path, { id: problemId.value }));
};

const handleSolutions = () => {
  // router.push(buildPath(routeMap.problem.path, { id: problemId.value }));
};
</script>

<template>
  <div class="container mx-auto max-w-6xl">
    <div class="card bg-base-100 shadow-xl">
      <div class="card-body">
        <!-- Loading State -->
        <div v-if="isLoading" class="flex items-center justify-center py-8">
          <span class="loading loading-spinner loading-lg"></span>
        </div>

        <!-- Problem Content -->
        <div v-else-if="problemData" class="space-y-6">
          <!-- Header -->
          <div class="flex items-start justify-between gap-4">
            <div class="flex-1">
              <h1 class="text-3xl font-bold mb-2">
                {{ problemData.name }}
              </h1>
              <div class="flex items-center gap-4 text-sm text-base-content/70">
                <div class="flex items-center gap-1">
                  <Icon icon="fa7-solid:clock" width="14" />
                  <span>{{ problemData.timeLimit }}ms</span>
                </div>
                <div class="flex items-center gap-1">
                  <Icon icon="fa7-solid:memory" width="14" />
                  <span>{{ problemData.memLimit }}MB</span>
                </div>
                <div class="flex items-center gap-1">
                  <Icon icon="fa7-solid:hashtag" width="14" />
                  <span>{{ problemData.problemId }}</span>
                </div>
              </div>
            </div>
            <div class="flex gap-2">
              <button v-if="isAdminOrTeacher" class="btn btn-sm w-28" @click="handleEdit">
                <Icon icon="fa7-solid:pen-to-square" width="14" />
                Edit
              </button>
              <button class="btn btn-sm w-28" @click="handleSolutions">
                <Icon icon="fa7-solid:lightbulb" width="14" />
                Solutions
              </button>
              <button class="btn btn-sm btn-primary w-28" @click="handleSubmit">
                <Icon icon="fa7-solid:code" width="14" />
                Submit
              </button>
            </div>
          </div>

          <div class="divider"></div>

          <!-- Description -->
          <div class="space-y-4">
            <h2 class="text-xl font-semibold flex items-center gap-2">
              <Icon icon="fa7-solid:book" width="20" />
              Description
            </h2>
            <div class="prose max-w-none" v-html="renderMarkdown(problemData.description)"></div>
          </div>

          <div class="divider"></div>

          <!-- Input Description -->
          <div class="space-y-4">
            <h2 class="text-xl font-semibold flex items-center gap-2">
              <Icon icon="fa7-solid:arrow-right-to-bracket" width="20" />
              Input
            </h2>
            <div class="prose max-w-none" v-html="renderMarkdown(problemData.inputDescription)"></div>
          </div>

          <div class="divider"></div>

          <!-- Output Description -->
          <div class="space-y-4">
            <h2 class="text-xl font-semibold flex items-center gap-2">
              <Icon icon="fa7-solid:arrow-right-from-bracket" width="20" />
              Output
            </h2>
            <div class="prose max-w-none" v-html="renderMarkdown(problemData.outputDescription)"></div>
          </div>

          <!-- Sample Cases -->
          <div v-if="problemData.samples && problemData.samples.length > 0">
            <div class="divider"></div>
            <div class="space-y-4">
              <h2 class="text-xl font-semibold flex items-center gap-2">
                <Icon icon="fa7-solid:flask" width="20" />
                Sample Cases
              </h2>
              <div v-for="(sample, index) in problemData.samples" :key="index" class="space-y-2">
                <h3 class="font-semibold text-base">Sample {{ index + 1 }}</h3>
                <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
                  <div>
                    <div class="text-sm font-medium mb-1">Input:</div>
                    <pre
                      class="bg-base-200 p-3 rounded-lg overflow-x-auto text-sm"><code>{{ sample.input }}</code></pre>
                  </div>
                  <div>
                    <div class="text-sm font-medium mb-1">Output:</div>
                    <pre
                      class="bg-base-200 p-3 rounded-lg overflow-x-auto text-sm"><code>{{ sample.output }}</code></pre>
                  </div>
                </div>
              </div>
            </div>
          </div>

          <!-- Note (if exists) -->
          <div v-if="problemData.note">
            <div class="divider"></div>
            <div class="space-y-4">
              <h2 class="text-xl font-semibold flex items-center gap-2">
                <Icon icon="fa7-solid:note-sticky" width="20" />
                Note
              </h2>
              <div class="prose max-w-none" v-html="renderMarkdown(problemData.note)"></div>
            </div>
          </div>

        </div>
      </div>
    </div>
  </div>
</template>
