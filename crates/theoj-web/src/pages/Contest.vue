<script setup lang="ts">
import { Icon } from "@iconify/vue";
import { computed, onMounted, ref } from "vue";
import { useRoute, useRouter } from "vue-router";
import { useToast } from "vue-toastification";
import ConfirmModal from "@/components/Modal/modals/ConfirmModal.vue";
import InputModal from "@/components/Modal/modals/InputModal.vue";
import { useModal } from "@/components/Modal/useModal.mjs";
import { useApiErrorHandler } from "@/composables/useApiErrorHandler.mjs";
import { useMarkdownRenderer } from "@/composables/useMarkdownRenderer.mts";
import { buildPath, routeMap } from "@/routes.mjs";
import { useContestPasswordStore } from "@/stores/contestPassword.mjs";
import { useUserStore } from "@/stores/user.mjs";
import type {
  GetAcStatusResponse,
  GetContestRankingResponse,
  GetContestResponse,
} from "@/theoj-api";
import {
  ContestService,
  ContestStatus,
  ProblemService,
  SubmissionResult,
  UserRole,
  UserService,
} from "@/theoj-api";

const route = useRoute();
const router = useRouter();
const toast = useToast();
const { handleApiError } = useApiErrorHandler();
const { renderMarkdown } = useMarkdownRenderer();
const contestPasswordStore = useContestPasswordStore();
const userStore = useUserStore();

const contestId = computed(() => route.params.id as string);
const isLoading = ref(true);
const contestData = ref<GetContestResponse | null>(null);
const currentUserRole = ref<UserRole | null>(null);
const activeTab = ref<"problems" | "ranking">("problems");
const rankingData = ref<GetContestRankingResponse | null>(null);
const problemStatuses = ref<Map<string, GetAcStatusResponse>>(new Map());

const isAdminOrTeacher = computed(() => {
  return (
    currentUserRole.value === UserRole.ADMIN ||
    currentUserRole.value === UserRole.TEACHER
  );
});

const promptForPassword = (isWrongPassword = false) => {
  const { open, close } = useModal({
    component: InputModal,
    attrs: {
      title: "Contest Password Required",
      placeholder: "Enter contest password",
      inputType: "password",
      confirmText: "Submit",
      cancelText: "Cancel",
      errorMessage: isWrongPassword
        ? "Incorrect password. Please try again."
        : "",
      initialValue: contestId.value
        ? contestPasswordStore.getPassword(Number(contestId.value)) || ""
        : "",
      async onConfirm(password: string) {
        if (contestId.value) {
          close();
          await loadContestData(contestId.value, password);
        }
      },
      onCancel() {
        router.push(routeMap.contestList.path);
        close();
      },
    },
  });
  open();
};

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
      "<p>Are you sure you want to delete this contest? This action cannot be undone.</p>"
  },
});

const loadContestData = async (id: string, password?: string) => {
  try {
    isLoading.value = true;

    const storedPassword =
      password || contestPasswordStore.getPassword(Number(id));

    const response = await ContestService.getContest(
      id,
      storedPassword || null,
    );
    contestData.value = response;

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

    // Load problem statuses if user is logged in
    if (userStore.userId && response.problemIds.length > 0) {
      await loadProblemStatuses(response.problemIds);
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

const loadProblemStatuses = async (problemIds: number[]) => {
  const promises = problemIds.map(async (id) => {
    try {
      const status = await ProblemService.getAcStatus(id.toString());
      problemStatuses.value.set(id.toString(), status);
    } catch (e) {
      console.error(`Failed to load status for problem ${id}:`, e);
    }
  });

  await Promise.all(promises);
};

const loadRankingData = async () => {
  if (!contestId.value) return;

  try {
    const storedPassword = contestPasswordStore.getPassword(
      Number(contestId.value),
    );
    const response = await ContestService.getContestRanking(
      contestId.value,
      storedPassword || null,
    );
    rankingData.value = response;
  } catch (e) {
    handleApiError(e);
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

const getProblemStatusIcon = (problemId: string) => {
  const status = problemStatuses.value.get(problemId);
  if (!status) return null;

  if (status.status === SubmissionResult.ACCEPTED) {
    return { icon: "fa6-solid:circle-check", color: "text-success" };
  } else if (status.tried) {
    return { icon: "fa6-solid:circle-xmark", color: "text-error" };
  }
  return null;
};

const switchTab = async (tab: "problems" | "ranking") => {
  activeTab.value = tab;
  if (tab === "ranking" && !rankingData.value) {
    await loadRankingData();
  }
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

    <div v-else-if="contestData" class="space-y-8">
      <!-- Contest Info Card -->
      <div class="card bg-base-100 shadow-xl">
        <div class="card-body">
          <div class="flex justify-between items-start">
            <div class="flex-1">
              <h2 class="card-title text-3xl mb-2">
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
      <div class="card bg-base-100 shadow-xl">
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
                    <th>Problem ID</th>
                    <th>Status</th>
                    <th class="w-32">Action</th>
                  </tr>
                </thead>
                <tbody>
                  <tr v-for="(problemId, index) in contestData.problemIds" :key="problemId">
                    <td>{{ String.fromCharCode(65 + index) }}</td>
                    <td>{{ problemId }}</td>
                    <td>
                      <template v-if="getProblemStatusIcon(problemId.toString())">
                        <!-- TODO: does this icon need 'inline-flex items-center'? -->
                        <Icon :icon="getProblemStatusIcon(problemId.toString())!.icon"
                          :class="['text-xl', getProblemStatusIcon(problemId.toString())!.color]" />
                      </template>
                      <span v-else class="text-sm opacity-50">Not attempted</span>
                    </td>
                    <td>
                      <RouterLink
                        :to="buildPath(routeMap.contestProblem.path, { contestId: contestId.toString(), problemId: problemId.toString() })"
                        class="btn btn-sm btn-primary">
                        View
                      </RouterLink>
                    </td>
                  </tr>
                </tbody>
              </table>
            </div>
          </div>

          <!-- Ranking Tab -->
          <div v-if="activeTab === 'ranking'" class="mt-4">
            <div v-if="!rankingData" class="flex justify-center py-8">
              <span class="loading loading-spinner loading-lg"></span>
            </div>
            <div v-else class="overflow-x-auto">
              <table class="table">
                <thead>
                  <tr>
                    <th>Rank</th>
                    <th>User</th>
                    <th>Solved</th>
                    <th>Penalty</th>
                    <th v-for="(_, index) in contestData.problemIds" :key="index">
                      {{ String.fromCharCode(65 + index) }}
                    </th>
                  </tr>
                </thead>
                <tbody>
                  <tr v-for="(item, index) in rankingData.rankings" :key="item.userId">
                    <td>{{ index + 1 }}</td>
                    <td>
                      <RouterLink :to="buildPath(routeMap.profile.path, { id: item.userId })" class="link link-primary">
                        {{ item.username }}
                      </RouterLink>
                    </td>
                    <td>{{ item.solvedCount }}</td>
                    <td>{{ item.totalPenalty }}</td>
                    <td v-for="result in item.problemResults" :key="result.problemId">
                      <!--TODO: do these icon need "inline-flex items-center"?-->
                      <div v-if="result.accepted" class="text-success text-center">
                        <Icon icon="fa6-solid:check" class="text-xl" />
                        <div class="text-xs">{{ result.attempts }}</div>
                      </div>
                      <div v-else-if="result.attempts > 0" class="text-error text-center">
                        <Icon icon="fa6-solid:xmark" class="text-xl" />
                        <div class="text-xs">{{ result.attempts }}</div>
                      </div>
                      <div v-else class="text-center opacity-30">-</div>
                    </td>
                  </tr>
                </tbody>
              </table>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
