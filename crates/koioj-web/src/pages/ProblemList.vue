<script setup lang="ts">
import { Icon } from "@iconify/vue";
import { computed, onMounted, ref, watch } from "vue";
import { useRoute, useRouter } from "vue-router";
import { useToast } from "vue-toastification";
import ProblemStatusBadge from "@/components/Badges/ProblemStatusBadge.vue";
import EntityLink from "@/components/EntityLink.vue";
import Pagination from "@/components/Pagination.vue";
import { useApiErrorHandler } from "@/composables/useApiErrorHandler.mjs";
import {
  type ListProblemsResponse,
  ProblemService,
  UserRole,
  UserService,
} from "@/koioj-api";
import { routeMap } from "@/routes.mjs";
import { useUserStore } from "@/stores/user.mjs";

const { handleApiError } = useApiErrorHandler();
const router = useRouter();
const route = useRoute();
const toast = useToast();
const userStore = useUserStore();

const currentUserRole = ref<UserRole | null>(null);
const problemsData = ref<ListProblemsResponse | null>(null);
const isLoading = ref(true);

const currentPage = ref(parseInt(route.query.page as string, 10) || 1);
const pageSize = ref(parseInt(route.query.pageSize as string, 10) || 10);

const canAddProblem = computed(() => {
  return (
    currentUserRole.value === UserRole.ADMIN ||
    currentUserRole.value === UserRole.TEACHER
  );
});

const totalPages = computed(() => {
  if (!problemsData.value?.total) return 0;
  return Math.ceil(problemsData.value.total / pageSize.value);
});

const loadProblems = async () => {
  isLoading.value = true;
  try {
    const roleResponse = await UserService.getRole(userStore.userId ?? -1);
    currentUserRole.value = roleResponse.role;

    const response = await ProblemService.listProblems(
      currentPage.value,
      pageSize.value,
    );
    problemsData.value = response;

    toast.success("Problems loaded!");
  } catch (e) {
    handleApiError(e);
  } finally {
    isLoading.value = false;
  }
};

watch(
  () => route.query,
  (newQuery) => {
    const page = parseInt(newQuery.page as string, 10) || 1;
    const size = parseInt(newQuery.pageSize as string, 10) || 10;

    if (currentPage.value !== page || pageSize.value !== size) {
      currentPage.value = page;
      pageSize.value = size;
      loadProblems();
    }
  }
);

onMounted(() => {
  loadProblems();
});

const handleAddProblem = () => {
  router.push(routeMap.createProblem.path);
};

const handlePageChange = (page: number) => {
  router.push({
    query: {
      ...route.query,
      page: page.toString(),
      pageSize: pageSize.value.toString(),
    },
  });
};
</script>

<template>
  <div class="container mx-auto max-w-6xl">
    <div class="card bg-base-100 shadow-xl">
      <div class="card-body">
        <div class="flex items-center justify-between mb-4">
          <h2 class="card-title">
            Problems
          </h2>

          <button v-if="canAddProblem" class="btn btn-primary" @click="handleAddProblem">
            <Icon icon="fa6-solid:plus" width="16" />
            Add Problem
          </button>
        </div>

        <div v-if="isLoading" class="flex items-center justify-center py-8">
          <span class="loading loading-spinner loading-lg"></span>
        </div>

        <!-- Problems Table -->
        <div v-else-if="problemsData?.problems.length" class="overflow-x-auto">
          <table class="table table-zebra">
            <thead>
              <tr>
                <th>ID</th>
                <th>Status</th>
                <th>Name</th>
                <th></th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="problem in problemsData.problems" :key="problem.problemId">
                <td>{{ problem.problemId }}</td>
                <td>
                  <ProblemStatusBadge :problem-id="problem.problemId" />
                </td>
                <td>
                  <EntityLink entity-type="problem" :entity-id="problem.problemId" display-type="link">
                    {{ problem.name }}
                  </EntityLink>
                </td>
                <td class="text-right">
                  <EntityLink entity-type="problem" :entity-id="problem.problemId" display-type="button" />
                </td>
              </tr>
            </tbody>
          </table>
        </div>


        <!-- Empty -->
        <div v-else class="flex flex-col items-center justify-center py-12 text-base-content/70">
          <Icon icon="fa6-solid:inbox" width="48" class="mb-4" />
          <p>No problems found</p>
        </div>

        <Pagination v-if="!isLoading && problemsData?.problems.length" :current-page="currentPage"
          :last-page="totalPages" @page-change="handlePageChange" />

        <!-- Total Info -->
        <div v-if="!isLoading && problemsData?.problems.length" class="text-center text-sm text-base-content/70 mt-2">
          Page {{ currentPage }} of {{ totalPages }} (Total:
          {{ problemsData.total }} problems)
        </div>
      </div>
    </div>
  </div>
</template>
