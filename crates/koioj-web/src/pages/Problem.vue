<script setup lang="ts">
import { Icon } from "@iconify/vue";
import { computed, onMounted, ref } from "vue";
import { useRoute, useRouter } from "vue-router";
import { useToast } from "vue-toastification";
import SubmissionResultBadge from "@/components/Badges/SubmissionResultBadge.vue";
import EntityLink from "@/components/EntityLink.vue";
import ConfirmModal from "@/components/Modal/modals/ConfirmModal.vue";
import { useModal } from "@/components/Modal/useModal.mjs";
import Pagination from "@/components/Pagination.vue";
import { useApiErrorHandler } from "@/composables/useApiErrorHandler.mjs";
import { useMarkdownRenderer } from "@/composables/useMarkdownRenderer.mts";
import {
  type GetProblemResponse,
  type ListSubmissionsResponse,
  ProblemService,
  UserRole,
  UserService,
} from "@/koioj-api";
import { buildPath, routeMap } from "@/routes.mjs";
import { useUserStore } from "@/stores/user.mjs";
import { APP_NAME, parseIntOrNull } from "@/utils.mjs";

const { handleApiError } = useApiErrorHandler();
const { renderMarkdown } = useMarkdownRenderer();
const router = useRouter();
const route = useRoute();
const toast = useToast();
const userStore = useUserStore();
const submissions = ref<ListSubmissionsResponse | null>(null);
const currentPage = ref(1);
const pageSize = 10;
const isLoadingSubmissions = ref(false);

const problemId = computed(() => {
  // contest mode: /contest/:cid/problem/:pid
  if (route.params.problemId) {
    return parseIntOrNull(route.params.problemId) ?? -1;
  }
  // normal mode: /problem/:id
  return parseIntOrNull(route.params.id) ?? -1;
});

const contestId = computed(() => parseIntOrNull(route.params.contestId));
const isContestMode = computed(() => !!contestId.value);

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
      UserService.getRole(userStore.userId ?? -1),
      ProblemService.getProblem(problemId.value),
    ]);

    currentUserRole.value = roleResponse.role;
    problemData.value = problemResponse;

    document.title = `${problemResponse.name} - ${APP_NAME}`;
  } catch (e) {
    handleApiError(e);
    router.push(routeMap.index.path);
  } finally {
    isLoading.value = false;
  }
};

const handleEdit = () => {
  router.push(buildPath(routeMap.editProblem.path, { id: problemId.value }));
};

const { open: handleDelete, close: closeDeleteModal } = useModal({
  component: ConfirmModal,
  attrs: {
    title: "Are you sure to delete this problem?",
    reverseColors: true,
    reverseOrder: true,
    async onYes() {
      try {
        await ProblemService.deleteProblem(problemId.value);
        toast.success("Problem deleted successfully!");
        router.push(routeMap.problemList.path);
      } catch (e) {
        handleApiError(e);
      }
    },
    onNo() { },
  },
  slots: {
    default:
      "<p>Are you sure you want to delete this problem? This action cannot be undone.</p>",
  },
});

const handleSubmit = () => {
  console.log(isContestMode.value);
  if (isContestMode.value) {
    if (!contestId.value) {
      toast.error("invalid contest!");
      return;
    }
    router.push(
      buildPath(routeMap.contestSubmit.path, {
        contestId: contestId.value,
        problemId: problemId.value,
      }),
    );
    return;
  }
  router.push(buildPath(routeMap.submit.path, { id: problemId.value }));
};

const loadSubmissions = async (page: number = 1) => {
  isLoadingSubmissions.value = true;
  try {
    const response = await ProblemService.listSubmissions(
      problemId.value,
      page,
      pageSize,
      contestId.value,
    );
    submissions.value = response;
    currentPage.value = page;
  } catch (e) {
    handleApiError(e);
  } finally {
    isLoadingSubmissions.value = false;
  }
};
const totalPages = computed(() => {
  if (!submissions.value) return 1;
  return Math.ceil(submissions.value.total / pageSize);
});

const handlePageChange = (page: number) => {
  loadSubmissions(page);
};

const handleSolutions = () => {
  router.push(buildPath(routeMap.soloutionList.path, { id: problemId.value }));
};

onMounted(async () => {
  await loadProblemData();
  await loadSubmissions(1);
});
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
              <button class="btn w-30" @click="handleSolutions">
                <Icon icon="fa7-solid:lightbulb" width="14" />
                Solutions
              </button>
              <button class="btn btn-primary w-28" @click="handleSubmit">
                <Icon icon="fa7-solid:code" width="14" />
                Submit
              </button>
              <button v-if="isAdminOrTeacher" @click="handleEdit" class="btn btn-warning">
                <Icon icon="fa6-solid:pen-to-square" />
                Edit
              </button>
              <button v-if="isAdminOrTeacher" @click="handleDelete" class="btn btn-error">
                <Icon icon="fa6-solid:trash" />
                Delete
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

    <div class="card bg-base-100 shadow-xl mt-6">
      <div class="card-body">
        <h2 class="card-title flex items-center gap-2 mb-4">
          <Icon icon="fa7-solid:list" width="20" />
          Recent Submissions
        </h2>
        <!-- Loading State -->
        <div v-if="isLoadingSubmissions" class="flex items-center justify-center py-8">
          <span class="loading loading-spinner loading-lg"></span>
        </div>
        <!-- Submissions Table -->
        <div v-else-if="submissions && submissions.submissions.length > 0" class="space-y-4">
          <div class="overflow-x-auto">
            <table class="table table-zebra w-full">
              <thead>
                <tr>
                  <th>ID</th>
                  <th v-if="isAdminOrTeacher">User</th>
                  <th>Result</th>
                  <th>Language</th>
                  <th>Time</th>
                  <th>Memory</th>
                  <th>Submit Time</th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="submission in submissions.submissions" :key="submission.submissionId">
                  <td>
                    <EntityLink :entity-type="isContestMode ? 'contestSubmission' : 'submission'" display-type="link"
                      :entity-id="submission.submissionId" :problem-id="submission.problemId"
                      :contest-id="contestId ?? undefined" />
                  </td>
                  <td v-if="isAdminOrTeacher">
                    <EntityLink entity-type="user" display-type="link" :entity-id="submission.userId">
                      {{ submission.username }}
                    </EntityLink>
                  </td>
                  <td>
                    <SubmissionResultBadge :result="submission.result" />
                  </td>
                  <td>{{ submission.lang }}</td>
                  <td>{{ submission.timeConsumption ?? '-' }}ms</td>
                  <td>{{ submission.memConsumption ?? '-' }}MB</td>
                  <td>{{ new Date(submission.createdAt).toLocaleString() }}</td>
                </tr>
              </tbody>
            </table>
          </div>
          <!-- Pagination -->
          <div class="flex justify-center mt-4">
            <Pagination :current-page="currentPage" :last-page="totalPages" @page-change="handlePageChange" />
          </div>
        </div>
        <!-- Empty State -->
        <div v-else class="text-center py-8 text-base-content/70">
          <Icon icon="fa7-solid:inbox" width="48" class="mx-auto mb-2 opacity-50" />
          <p>No submissions yet</p>
        </div>
      </div>
    </div>
  </div>
</template>