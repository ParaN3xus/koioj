<script setup lang="ts">
import { Icon } from "@iconify/vue";
import { computed, onMounted, ref } from "vue";
import { useRoute, useRouter } from "vue-router";
import EntityLink from "@/components/EntityLink.vue";
import { useApiErrorHandler } from "@/composables/useApiErrorHandler.mjs";
import { buildPath, routeMap } from "@/routes.mjs";
import { useUserStore } from "@/stores/user.mjs";
import type { GetProblemResponse, SolutionListItem } from "@/theoj-api";
import { ProblemService, UserRole, UserService } from "@/theoj-api";
import { parseIntOrNull } from "@/utils.mjs";

const route = useRoute();
const router = useRouter();
const userStore = useUserStore();
const { handleApiError } = useApiErrorHandler();

const problemId = parseIntOrNull(route.params.id) ?? -1;
const problem = ref<GetProblemResponse | null>(null);
const solutions = ref<SolutionListItem[]>([]);
const loading = ref(true);
const currentUserRole = ref<UserRole>(UserRole.STUDENT);

const isAdminOrTeacher = computed(() => {
  return (
    currentUserRole.value === UserRole.ADMIN ||
    currentUserRole.value === UserRole.TEACHER
  );
});

const fetchData = async () => {
  try {
    loading.value = true;
    const [problemResponse, solutionsResponse, roleResponse] =
      await Promise.all([
        ProblemService.getProblem(problemId),
        ProblemService.listSolutions(problemId),
        UserService.getRole(userStore.userId ?? -1),
      ]);

    problem.value = problemResponse;
    solutions.value = solutionsResponse.solutions;
    currentUserRole.value = roleResponse.role;

    document.title = `Solutions of ${problemResponse.name} - TheOJ`;
  } catch (e) {
    handleApiError(e);
  } finally {
    loading.value = false;
  }
};

const handleAddSolution = () => {
  router.push(buildPath(routeMap.createSolution.path, { id: problemId }));
};

const handleViewSolution = (solutionId: string) => {
  console.log(
    buildPath(routeMap.solution.path, {
      problemId: problemId,
      solutionId: solutionId,
    }),
  );
  router.push(
    buildPath(routeMap.solution.path, {
      problemId: problemId,
      solutionId: solutionId,
    }),
  );
};

onMounted(() => {
  fetchData();
});
</script>

<template>
  <div class="container mx-auto max-w-6xl p-4">
    <div class="card bg-base-100 shadow-xl">
      <div class="card-body">
        <!-- Header -->
        <div class="flex items-center justify-between mb-6">
          <h1 class="text-3xl font-bold" v-if="problem">
            {{ problem.name }} - Solutions
          </h1>

          <button v-if="isAdminOrTeacher" @click="handleAddSolution" class="btn btn-primary">
            <Icon icon="fa7-solid:plus" class="w-5 h-5" />
            Add Solution
          </button>
        </div>

        <!-- Loading State -->
        <div v-if="loading" class="flex justify-center py-12">
          <span class="loading loading-spinner loading-lg"></span>
        </div>

        <!-- Empty State -->
        <div v-else-if="solutions.length === 0" class="text-center py-12">
          <Icon icon="fa6-solid:book-open" class="w-16 h-16 mx-auto text-base-300 mb-4" />
          <p class="text-lg text-base-content/70">No solutions yet</p>
        </div>

        <!-- Solutions List -->
        <div v-else class="space-y-4 overflow-x-auto">
          <table class="table table-zebra">
            <thead>
              <tr>
                <th>Title</th>
                <th>Author</th>
                <th>Created At</th>
                <th></th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="solution in solutions" :key="solution.solutionId">
                <td>
                  <EntityLink entity-type="solution" :entity-id="solution.solutionId" :problem-id="problem?.problemId"
                    display-type="link">
                    {{ solution.title }}
                  </EntityLink>
                </td>
                <td>
                  <EntityLink entity-type="user" :entity-id="solution.authorId" display-type="link">
                    {{ solution.authorName }}
                  </EntityLink>
                </td>
                <td>
                  {{ new Date(solution.createdAt).toLocaleDateString() }}
                </td>
                <td class="text-right">
                  <EntityLink entity-type="solution" :entity-id="solution.solutionId" :problem-id="problem?.problemId"
                    display-type="button" />
                </td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>
    </div>
  </div>
</template>
