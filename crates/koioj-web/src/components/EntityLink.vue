<script setup lang="ts">
import { Icon } from "@iconify/vue";
import { computed, onMounted, ref, useSlots } from "vue";
import { RouterLink } from "vue-router";
import { useApiErrorHandler } from "@/composables/useApiErrorHandler.mjs";
import {
  ContestService,
  ProblemService,
  TrainingPlanService,
  UserService,
} from "@/koioj-api";
import { buildPath, routeMap } from "@/routes.mjs";

type EntityType =
  | "user"
  | "problem"
  | "solution"
  | "submission"
  | "contest"
  | "contestProblem"
  | "contestSubmission"
  | "trainingPlan";
type DisplayType = "button" | "link";

interface Props {
  entityType: EntityType;
  displayType?: DisplayType;
  entityId: number;
  problemId?: number; // For solution, submission, contestProblem, contestSubmission
  contestId?: number; // For contestProblem, contestSubmission
  customClass?: string;
}

const props = withDefaults(defineProps<Props>(), {
  displayType: "link",
});

const slots = useSlots();
const { handleApiError } = useApiErrorHandler();
const entityName = ref<string>("");
const isLoading = ref(false);

const entityPath = computed(() => {
  const params: Record<string, string | number> = {};

  switch (props.entityType) {
    case "user":
      params.id = props.entityId;
      return buildPath(routeMap.profile.path, params);

    case "problem":
      params.id = props.entityId;
      return buildPath(routeMap.problem.path, params);

    case "solution":
      if (!props.problemId)
        throw new Error("problemId is required for solution");
      params.problemId = props.problemId;
      params.solutionId = props.entityId;
      return buildPath(routeMap.solution.path, params);

    case "submission":
      if (!props.problemId)
        throw new Error("problemId is required for submission");
      params.problemId = props.problemId;
      params.submissionId = props.entityId;
      return buildPath(routeMap.submission.path, params);

    case "contest":
      params.id = props.entityId;
      return buildPath(routeMap.contest.path, params);

    case "contestProblem":
      if (!props.contestId)
        throw new Error("contestId is required for contestProblem");
      params.contestId = props.contestId;
      params.problemId = props.entityId;
      return buildPath(routeMap.contestProblem.path, params);

    case "contestSubmission":
      if (!props.contestId || !props.problemId) {
        throw new Error(
          "contestId and problemId are required for contestSubmission",
        );
      }
      params.contestId = props.contestId;
      params.problemId = props.problemId;
      params.submissionId = props.entityId;
      return buildPath(routeMap.contestSubmission.path, params);

    case "trainingPlan":
      params.id = props.entityId;
      return buildPath(routeMap.trainingPlan.path, params);

    default:
      return "/";
  }
});

const defaultClass = computed(() => {
  if (props.displayType === "button") {
    return "btn btn-ghost btn-sm";
  }
  return "link link-primary font-semibold";
});

const finalClass = computed(() => {
  return props.customClass ?? defaultClass.value;
});

const hasDefaultSlot = computed(() => {
  return !!slots.default;
});

const fetchEntityName = async () => {
  if (props.displayType === "button" || hasDefaultSlot.value) return;

  isLoading.value = true;
  try {
    switch (props.entityType) {
      case "user": {
        const response = await UserService.getProfile(props.entityId);
        entityName.value = response.username;
        break;
      }

      case "problem":
      case "contestProblem": {
        const response = await ProblemService.getProblem(props.entityId);
        entityName.value = response.name;
        break;
      }

      case "contest": {
        const response = await ContestService.getContest(props.entityId);
        entityName.value = response.name;
        break;
      }

      case "trainingPlan": {
        const response = await TrainingPlanService.getTrainingPlan(
          props.entityId,
        );
        entityName.value = response.name;
        break;
      }

      case "solution": {
        if (!props.problemId)
          throw new Error("problemId is required for solution");
        const response = await ProblemService.getSolution(
          props.problemId,
          props.entityId,
        );
        entityName.value = response.title;
        break;
      }

      case "submission":
      case "contestSubmission": {
        // These types don't have direct service methods, fallback to default
        entityName.value = `${props.entityId}`;
        break;
      }
    }
  } catch (error) {
    handleApiError(error);
    entityName.value = `#${props.entityId}`;
  } finally {
    isLoading.value = false;
  }
};

onMounted(() => {
  fetchEntityName();
});
</script>

<template>
  <RouterLink v-if="displayType === 'button'" :to="entityPath" :class="finalClass">
    <slot>
      <Icon icon="fa6-solid:arrow-right" width="16" />
    </slot>
  </RouterLink>
  <RouterLink v-else :to="entityPath" :class="finalClass">
    <slot>
      <span v-if="isLoading">Loading...</span>
      <span v-else>{{ entityName || `${entityId}` }}</span>
    </slot>
  </RouterLink>
</template>
