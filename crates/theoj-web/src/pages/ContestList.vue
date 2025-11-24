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
  ContestService,
  ContestType,
  type ListContestsResponse,
  UserRole,
  UserService,
} from "@/theoj-api";

const { handleApiError } = useApiErrorHandler();
const router = useRouter();
const toast = useToast();
const userStore = useUserStore();

const currentUserRole = ref<UserRole | null>(null);
const contestsData = ref<ListContestsResponse | null>(null);
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

const canManageContests = computed(() => {
  return (
    currentUserRole.value === UserRole.ADMIN ||
    currentUserRole.value === UserRole.TEACHER
  );
});

const totalPages = computed(() => {
  if (!contestsData.value?.total) return 0;
  return Math.ceil(contestsData.value.total / pageSize.value);
});

const loadContests = async () => {
  isLoading.value = true;
  try {
    const roleResponse = await UserService.getRole(userStore.userId);
    currentUserRole.value = roleResponse.role;

    const response = await ContestService.listContests(
      currentPage.value,
      pageSize.value,
      endAfter.value,
    );
    contestsData.value = response;

    toast.success("Contests loaded!");
  } catch (e) {
    handleApiError(e);
  } finally {
    isLoading.value = false;
  }
};

onMounted(() => {
  loadContests();
});

const handleAddContest = () => {
  router.push(routeMap.createContest.path);
};

const handleViewContest = (contestId: string) => {
  router.push(buildPath(routeMap.contest.path, { id: contestId }));
};

const handlePageChange = (page: number) => {
  currentPage.value = page;
  loadContests();
};

const handleFilterChange = () => {
  currentPage.value = 1;
  loadContests();
};

const formatDateTime = (dateTime: string) => {
  return new Date(dateTime).toLocaleString();
};

const getContestTypeColor = (type: ContestType) => {
  return type === ContestType.PUBLIC ? "badge-success" : "badge-warning";
};
</script>

<template>
  <div class="container mx-auto max-w-6xl">
    <div class="card bg-base-100 shadow-xl">
      <div class="card-body">
        <div class="flex items-center justify-between mb-4">
          <h2 class="card-title">
            Contests
          </h2>

          <button v-if="canManageContests" class="btn btn-primary" @click="handleAddContest">
            <Icon icon="fa6-solid:plus" width="16" />
            Add Contest
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

        <!-- Contests Table -->
        <div v-else-if="contestsData?.contests.length" class="overflow-x-auto">
          <table class="table table-zebra">
            <thead>
              <tr>
                <th>Name</th>
                <th v-if="canManageContests">Type</th>
                <th>Begin Time</th>
                <th>End Time</th>
                <th></th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="contest in contestsData.contests" :key="contest.contestId">
                <td>
                  <div class="flex items-center gap-2">
                    <EntityLink entity-type="contest" :entity-id="contest.contestId" display-type="link">
                      {{ contest.name }}
                    </EntityLink>
                    <Icon v-if="contest.hasPassword" icon="fa6-solid:lock" width="14" class="text-warning" />
                  </div>
                </td>
                <td v-if="canManageContests">
                  <span class="badge" :class="getContestTypeColor(contest.type)">
                    {{ contest.type }}
                  </span>
                </td>
                <td>{{ formatDateTime(contest.beginTime) }}</td>
                <td>{{ formatDateTime(contest.endTime) }}</td>
                <td class="text-right">
                  <EntityLink entity-type="contest" :entity-id="contest.contestId" display-type="button" />
                </td>
              </tr>
            </tbody>
          </table>
        </div>

        <!-- Empty -->
        <div v-else class="flex flex-col items-center justify-center py-12 text-base-content/70">
          <Icon icon="fa6-solid:inbox" width="48" class="mb-4" />
          <p>No contests found</p>
        </div>

        <Pagination v-if="!isLoading && contestsData?.contests.length" :current-page="currentPage"
          :last-page="totalPages" @page-change="handlePageChange" />

        <!-- Total Info -->
        <div v-if="!isLoading && contestsData?.contests.length" class="text-center text-sm text-base-content/70 mt-2">
          Page {{ currentPage }} of {{ totalPages }} (Total:
          {{ contestsData.total }} contests)
        </div>
      </div>
    </div>
  </div>
</template>
