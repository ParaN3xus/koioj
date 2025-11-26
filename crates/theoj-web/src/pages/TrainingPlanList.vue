<script setup lang="ts">
import { Icon } from "@iconify/vue";
import { computed, onMounted, ref } from "vue";
import { useRouter } from "vue-router";
import { useToast } from "vue-toastification";
import EntityLink from "@/components/EntityLink.vue";
import Pagination from "@/components/Pagination.vue";
import { useApiErrorHandler } from "@/composables/useApiErrorHandler.mjs";
import { buildPath, routeMap } from "@/routes.mjs";
import { useUserStore } from "@/stores/user.mjs";
import {
  type ListTrainingPlansResponse,
  TrainingPlanService,
  UserRole,
  UserService,
} from "@/theoj-api";

const { handleApiError } = useApiErrorHandler();
const router = useRouter();
const toast = useToast();
const userStore = useUserStore();

const currentUserRole = ref<UserRole | null>(null);
const trainingPlansData = ref<ListTrainingPlansResponse | null>(null);
const isLoading = ref(true);

const currentPage = ref(1);
const pageSize = ref(10);
const endAfter = ref<string>(new Date().toISOString());

const getDateValue = computed(() => {
  return endAfter.value.split("T")[0];
});
const handleDateInput = (event: Event) => {
  const target = event.target as HTMLInputElement;
  const dateStr = target.value;
  // date -> datatime at 0am today
  endAfter.value = new Date(`${dateStr}T00:00:00Z`).toISOString();
};

const canManageTrainingPlans = computed(() => {
  return (
    currentUserRole.value === UserRole.ADMIN ||
    currentUserRole.value === UserRole.TEACHER
  );
});

const totalPages = computed(() => {
  if (!trainingPlansData.value?.total) return 0;
  return Math.ceil(trainingPlansData.value.total / pageSize.value);
});

const loadTrainingPlans = async () => {
  isLoading.value = true;
  try {
    const roleResponse = await UserService.getRole(userStore.userId ?? -1);
    currentUserRole.value = roleResponse.role;

    const response = await TrainingPlanService.listTrainingPlans(
      currentPage.value,
      pageSize.value,
      endAfter.value,
    );
    trainingPlansData.value = response;

    toast.success("Training plans loaded!");
  } catch (e) {
    handleApiError(e);
  } finally {
    isLoading.value = false;
  }
};

onMounted(() => {
  loadTrainingPlans();
});

const handleAddTrainingPlan = () => {
  router.push(routeMap.createTrainingPlan.path);
};

const handleViewTrainingPlan = (trainingPlanId: string) => {
  router.push(buildPath(routeMap.trainingPlan.path, { id: trainingPlanId }));
};

const handlePageChange = (page: number) => {
  currentPage.value = page;
  loadTrainingPlans();
};

const handleFilterChange = () => {
  currentPage.value = 1;
  loadTrainingPlans();
};

</script>

<template>
  <div class="container mx-auto max-w-6xl">
    <div class="card bg-base-100 shadow-xl">
      <div class="card-body">
        <div class="flex items-center justify-between mb-4">
          <h2 class="card-title">
            Training Plans
          </h2>

          <button v-if="canManageTrainingPlans" class="btn btn-primary" @click="handleAddTrainingPlan">
            <Icon icon="fa6-solid:plus" width="16" />
            Add Training Plan
          </button>
        </div>

        <!-- Filter -->
        <div class="form-control w-full max-w-xs mb-4">
          <label class="label">
            <span class="label-text">End After</span>
          </label>
          <input :value="getDateValue" type="date" class="input input-bordered w-full max-w-xs"
            @change="handleDateInput($event); handleFilterChange()" />
        </div>

        <div v-if="isLoading" class="flex items-center justify-center py-8">
          <span class="loading loading-spinner loading-lg"></span>
        </div>

        <!-- Training Plans Table -->
        <div v-else-if="trainingPlansData?.plans.length" class="overflow-x-auto">
          <table class="table table-zebra">
            <thead>
              <tr>
                <th>ID</th>
                <th>Name</th>
                <th></th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="plan in trainingPlansData.plans" :key="plan.id">
                <td>
                  {{ plan.id }}
                </td>
                <td>
                  <EntityLink entity-type="trainingPlan" :entity-id="plan.id" display-type="link">
                    {{ plan.name }}
                  </EntityLink>
                </td>
                <td class="text-right">
                  <EntityLink entity-type="trainingPlan" :entity-id="plan.id" display-type="button" />
                </td>
              </tr>
            </tbody>
          </table>
        </div>

        <!-- Empty -->
        <div v-else class="flex flex-col items-center justify-center py-12 text-base-content/70">
          <Icon icon="fa6-solid:inbox" width="48" class="mb-4" />
          <p>No training plans found</p>
        </div>

        <Pagination v-if="!isLoading && trainingPlansData?.plans.length" :current-page="currentPage"
          :last-page="totalPages" @page-change="handlePageChange" />

        <!-- Total Info -->
        <div v-if="!isLoading && trainingPlansData?.plans.length" class="text-center text-sm text-base-content/70 mt-2">
          Page {{ currentPage }} of {{ totalPages }} (Total:
          {{ trainingPlansData.total }} training plans)
        </div>
      </div>
    </div>
  </div>
</template>
