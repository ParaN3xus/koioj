<script setup lang="ts">
import { Icon } from "@iconify/vue";
import { computed, onMounted, ref } from "vue";
import { useRoute, useRouter } from "vue-router";
import { useToast } from "vue-toastification";
import ContestStatusBadge from "@/components/Badges/ContestStatusBadge.vue";
import EntityLink from "@/components/EntityLink.vue";
import ConfirmModal from "@/components/Modal/modals/ConfirmModal.vue";
import { useModal } from "@/components/Modal/useModal.mjs";
import { useApiErrorHandler } from "@/composables/useApiErrorHandler.mjs";
import { useMarkdownRenderer } from "@/composables/useMarkdownRenderer.mts";
import { buildPath, routeMap } from "@/routes.mjs";
import { useUserStore } from "@/stores/user.mjs";
import type {
  GetTrainingPlanResponse,
} from "@/theoj-api";
import {
  TrainingPlanService,
  UserRole,
  UserService,
} from "@/theoj-api";
import { formatDateTime } from "@/utils.mjs";

const route = useRoute();
const router = useRouter();
const toast = useToast();
const { handleApiError } = useApiErrorHandler();
const { renderMarkdown } = useMarkdownRenderer();
const userStore = useUserStore();

const trainingPlanId = computed(() => route.params.id as string);
const isLoading = ref(true);
const trainingPlanData = ref<GetTrainingPlanResponse | null>(null);
const currentUserRole = ref<UserRole | null>(null);

const isAdminOrTeacher = computed(() => {
  return (
    currentUserRole.value === UserRole.ADMIN ||
    currentUserRole.value === UserRole.TEACHER
  );
});

const { open: handleDeleteTrainingPlan, close: closeDeleteTrainingPlanModal } =
  useModal({
    component: ConfirmModal,
    attrs: {
      title: "Are you sure to delete this training plan?",
      reverseColors: true,
      reverseOrder: true,
      async onYes() {
        try {
          await TrainingPlanService.deleteTrainingPlan(
            parseInt(trainingPlanId.value, 10),
          );
          toast.success("Training plan deleted successfully!");
          router.push(routeMap.trainingPlanList.path);
        } catch (e) {
          handleApiError(e);
        }
      },
      onNo() { },
    },
    slots: {
      default:
        "<p>Are you sure you want to delete this training plan? This action cannot be undone.</p>",
    },
  });

const loadTrainingPlanData = async (id: string) => {
  try {
    isLoading.value = true;

    const response = await TrainingPlanService.getTrainingPlan(
      parseInt(id, 10),
    );
    trainingPlanData.value = response;

    // Load user role if logged in
    if (userStore.userId) {
      const roleResponse = await UserService.getRole(userStore.userId);
      currentUserRole.value = roleResponse.role;
    }

    isLoading.value = false;
  } catch (e: unknown) {
    handleApiError(e);
    isLoading.value = false;
  }
};

const handleEditContest = () => {
  if (!trainingPlanId.value) return;
  router.push(
    buildPath(routeMap.editTrainingPlan.path, { id: trainingPlanId.value }),
  );
};

onMounted(async () => {
  if (trainingPlanId.value) {
    await loadTrainingPlanData(trainingPlanId.value);
  }
});
</script>

<template>
  <div class="container mx-auto max-w-6xl">
    <div v-if="isLoading" class="flex justify-center items-center min-h-screen">
      <span class="loading loading-spinner loading-lg"></span>
    </div>

    <div v-else-if="trainingPlanData">
      <!-- Contest Info Card -->
      <div class="card bg-base-100 shadow-xl">
        <div class="card-body">
          <div class="flex justify-between items-start">
            <div class="flex-1">
              <h2 class="card-title text-3xl mb-2">
                {{ trainingPlanData.name }}
              </h2>
            </div>

            <div class="flex gap-2">
              <template v-if="isAdminOrTeacher">
                <button @click="handleEditContest" class="btn btn-warning">
                  <Icon icon="fa6-solid:pen-to-square" />
                  Edit
                </button>
                <button @click="handleDeleteTrainingPlan" class="btn btn-error">
                  <Icon icon="fa6-solid:trash" />
                  Delete
                </button>
              </template>
            </div>
          </div>

          <div class="divider"></div>

          <div class="prose max-w-none" v-html="renderMarkdown(trainingPlanData.description)"></div>
        </div>
      </div>

      <!-- Problems & Ranking Card -->
      <div class="card bg-base-100 shadow-xl mt-6">
        <div class="card-body">
          <div class="overflow-x-auto">
            <table class="table table-zebra">
              <thead>
                <tr>
                  <th>Name</th>
                  <th>Begin Time</th>
                  <th>End Time</th>
                  <th>Status</th>
                  <th></th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="contest in trainingPlanData.contests" :key="contest.contestId">
                  <td>
                    <div class="flex items-center gap-2">
                      <EntityLink entity-type="contest" :entity-id="contest.contestId" display-type="link">
                        {{ contest.name }}
                      </EntityLink>
                    </div>
                  </td>
                  <td>{{ formatDateTime(contest.beginTime) }}</td>
                  <td>{{ formatDateTime(contest.endTime) }}</td>
                  <td>
                    <ContestStatusBadge :begin-time="contest.beginTime" :end-time="contest.endTime" />
                  </td>
                  <td class="text-right">
                    <EntityLink entity-type="contest" :entity-id="contest.contestId" display-type="button" />
                  </td>
                </tr>
              </tbody>
            </table>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
