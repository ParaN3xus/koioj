<script setup lang="ts">
import { Icon } from "@iconify/vue";
import { computed, onMounted, ref } from "vue";
import { useRoute, useRouter } from "vue-router";
import { useToast } from "vue-toastification";
import ProblemStatusBadge from "@/components/Badges/ProblemStatusBadge.vue";
import ContestRanking from "@/components/ContestRanking.vue";
import EntityLink from "@/components/EntityLink.vue";
import ConfirmModal from "@/components/Modal/modals/ConfirmModal.vue";
import { useModal } from "@/components/Modal/useModal.mjs";
import { useApiErrorHandler } from "@/composables/useApiErrorHandler.mjs";
import { useContestPasswordPrompt } from "@/composables/useContestPasswordPrompt.mjs";
import { useMarkdownRenderer } from "@/composables/useMarkdownRenderer.mts";
import type { GetContestResponse } from "@/koioj-api";
import { ContestService, UserRole, UserService } from "@/koioj-api";
import { buildPath, routeMap } from "@/routes.mjs";
import { useContestPasswordStore } from "@/stores/contestPassword.mjs";
import { useUserStore } from "@/stores/user.mjs";
import { APP_NAME, parseIntOrNull } from "@/utils.mjs";

const route = useRoute();
const router = useRouter();
const toast = useToast();
const { handleApiError } = useApiErrorHandler();
const { renderMarkdown } = useMarkdownRenderer();
const contestPasswordStore = useContestPasswordStore();
const userStore = useUserStore();

const contestId = computed(() => parseIntOrNull(route.params.id) ?? -1);
const isLoading = ref(true);
const contestData = ref<GetContestResponse | null>(null);
const currentUserRole = ref<UserRole | null>(null);
const activeTab = ref<"problems" | "ranking">("problems");

const isAdminOrTeacher = computed(() => {
  return (
    currentUserRole.value === UserRole.ADMIN ||
    currentUserRole.value === UserRole.TEACHER
  );
});

const { promptForPassword } = useContestPasswordPrompt({
  contestId: Number(contestId.value),
  onPasswordSubmit: async (password: string) => {
    if (!contestId.value) {
      toast.error("invalid contestId");
      return;
    }
    await loadContestData(contestId.value, password);
  },
});

const { open: handleDeleteContest, close: closeDeleteContestModal } = useModal({
  component: ConfirmModal,
  attrs: {
    title: "Are you sure to delete your contest?",
    reverseColors: true,
    reverseOrder: true,
    async onYes() {
      try {
        await ContestService.deleteContest(contestId.value);
        toast.success("Contest deleted successfully!");
        router.push(routeMap.contestList.path);
      } catch (e) {
        handleApiError(e);
      }
    },
    onNo() { },
  },
  slots: {
    default:
      "<p>Are you sure you want to delete this contest? This action cannot be undone.</p>",
  },
});

const loadContestData = async (id: number, password?: string) => {
  try {
    isLoading.value = true;

    const storedPassword =
      password || contestPasswordStore.getPassword(Number(id));

    const response = await ContestService.getContest(
      id,
      storedPassword || null,
    );
    contestData.value = response;

    document.title = `${response.name} - ${APP_NAME}`;

    if (storedPassword && !password) {
      // Verify stored password is still valid
      contestPasswordStore.setPassword(Number(id), storedPassword);
    } else if (password) {
      contestPasswordStore.setPassword(Number(id), password);
    }

    // Load user role if logged in
    if (userStore.userId) {
      const roleResponse = await UserService.getRole(userStore.userId);
      currentUserRole.value = roleResponse.role;
    }

    isLoading.value = false;
  } catch (e: unknown) {
    const err = e as { status?: number; body?: string | { message?: string } };

    if (err.status === 403) {
      const body =
        typeof err.body === "string" ? JSON.parse(err.body) : err.body;
      const message = body?.message || "";

      if (message === "contest password required") {
        isLoading.value = false;
        promptForPassword(false);
        return;
      }

      if (message === "contest password wrong") {
        isLoading.value = false;
        if (contestId.value) {
          contestPasswordStore.clearPassword(Number(contestId.value));
        }
        promptForPassword(true);
        return;
      }
    }

    handleApiError(e);
    isLoading.value = false;
  }
};

const handleJoinContest = async () => {
  if (!contestId.value) return;

  try {
    const storedPassword = contestPasswordStore.getPassword(
      Number(contestId.value),
    );
    await ContestService.joinContest(contestId.value, {
      password: storedPassword || null,
    });
    toast.success("Successfully joined the contest!");
    await loadContestData(contestId.value);
  } catch (e) {
    handleApiError(e);
  }
};

const handleEditContest = () => {
  if (!contestId.value) return;
  router.push(buildPath(routeMap.editContest.path, { id: contestId.value }));
};

const switchTab = async (tab: "problems" | "ranking") => {
  activeTab.value = tab;
};

onMounted(async () => {
  if (contestId.value) {
    await loadContestData(contestId.value);
  }
});
</script>

<template>
  <div class="container mx-auto max-w-6xl">
    <div v-if="isLoading" class="flex justify-center items-center min-h-screen">
      <span class="loading loading-spinner loading-lg"></span>
    </div>

    <div v-else-if="contestData">
      <!-- Contest Info Card -->
      <div class="card bg-base-100 shadow-xl">
        <div class="card-body">
          <div class="flex justify-between items-start">
            <div class="flex-1">
              <h2 class="card-title text-3xl mb-2 font-bold">
                {{ contestData.name }}
              </h2>
              <div class="text-sm opacity-70 space-y-1">
                <p class="inline-flex items-center gap-1">
                  <Icon icon="fa6-solid:calendar-days" class="inline" />
                  Begin: {{ new Date(contestData.beginTime).toLocaleString() }}
                </p>
                <br />
                <p class="inline-flex items-center gap-1">
                  <Icon icon="fa6-solid:calendar-days" class="inline" />
                  End: {{ new Date(contestData.endTime).toLocaleString() }}
                </p>
              </div>
            </div>

            <div class="flex gap-2">
              <button v-if="userStore.isLoggedIn" @click="handleJoinContest" class="btn btn-primary">
                <Icon icon="fa6-solid:right-to-bracket" />
                Join Contest
              </button>

              <template v-if="isAdminOrTeacher">
                <button @click="handleEditContest" class="btn btn-warning">
                  <Icon icon="fa6-solid:pen-to-square" />
                  Edit
                </button>
                <button @click="handleDeleteContest" class="btn btn-error">
                  <Icon icon="fa6-solid:trash" />
                  Delete
                </button>
              </template>
            </div>
          </div>

          <div class="divider"></div>

          <div class="prose max-w-none" v-html="renderMarkdown(contestData.description)"></div>

        </div>
      </div>

      <!-- Problems & Ranking Card -->
      <div class="card bg-base-100 shadow-xl mt-6">
        <div class="card-body">
          <!-- Tabs -->
          <div role="tablist" class="tabs tabs-bordered">
            <a role="tab" :class="['tab inline-flex items-center gap-1', { 'tab-active': activeTab === 'problems' }]"
              @click="switchTab('problems')">
              <Icon icon="fa6-solid:list" />
              Problems
            </a>
            <a role="tab" :class="['tab inline-flex items-center gap-1', { 'tab-active': activeTab === 'ranking' }]"
              @click="switchTab('ranking')">
              <Icon icon="fa6-solid:ranking-star" />
              Ranking
            </a>
          </div>

          <!-- Problems Tab -->
          <div v-if="activeTab === 'problems'" class="mt-4">
            <div class="overflow-x-auto">
              <table class="table">
                <thead>
                  <tr>
                    <th class="w-16">#</th>
                    <th>Problem</th>
                    <th>Status</th>
                    <th></th>
                  </tr>
                </thead>
                <tbody>
                  <tr v-for="(problemId, index) in contestData.problemIds" :key="problemId">
                    <td>{{ String.fromCharCode(65 + index) }}</td>
                    <td>
                      <EntityLink entity-type="contestProblem" :entity-id="problemId" :contest-id="contestId" />
                    </td>
                    <td>
                      <ProblemStatusBadge :problem-id="problemId" :contest-id="contestId" />
                    </td>
                    <td class="text-right">
                      <EntityLink entity-type="contestProblem" :entity-id="problemId" :contest-id="contestId"
                        display-type="button" />
                    </td>
                  </tr>
                </tbody>
              </table>
            </div>
          </div>

          <!-- Ranking Tab -->
          <div v-if="activeTab === 'ranking'" class="mt-4">
            <ContestRanking :contest-ids="contestId" :problem-ids="contestData.problemIds" />
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
