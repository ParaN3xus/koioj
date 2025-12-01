<script setup lang="ts">
import { Icon } from "@iconify/vue";
import { onMounted, ref } from "vue";
import { useRoute, useRouter } from "vue-router";
import { useToast } from "vue-toastification";
import ContestSelector from "@/components/ContestSelector.vue";
import EntityLink from "@/components/EntityLink.vue";
import PreviewableTextEdit from "@/components/PreviewableTextEdit.vue";
import { useApiErrorHandler } from "@/composables/useApiErrorHandler.mjs";
import {
  type ContestInfo,
  ContestService,
  type CreateTrainingPlanRequest,
  TrainingPlanService,
} from "@/koioj-api";
import { buildPath, routeMap } from "@/routes.mjs";
import { useContestPasswordStore } from "@/stores/contestPassword.mjs";

const { handleApiError } = useApiErrorHandler();
const router = useRouter();
const route = useRoute();
const toast = useToast();

const isSubmitting = ref(false);
const isLoading = ref(false);
const showDescriptionPreview = ref(false);
const isEditMode = ref(false);
const planId = ref<string | null>(null);

const formData = ref<CreateTrainingPlanRequest>({
  name: "",
  description: "",
});

const selectedContests = ref<Array<ContestInfo>>([]);
const contestPasswordStore = useContestPasswordStore();

// Participant management
const participantInput = ref("");
const selectedParticipants = ref<Array<number>>([]);

// Parse participant input like "+1-10, +15, -8-12"
const parseParticipantInput = (input: string) => {
  const add: number[] = [];
  const remove: number[] = [];

  const parts = input.split(",").map((p) => p.trim());

  for (const part of parts) {
    if (!part) continue;

    const isAdd = part.startsWith("+");
    const isRemove = part.startsWith("-");

    if (!isAdd && !isRemove) {
      toast.error(`Invalid format: "${part}". Use +id or -id`);
      continue;
    }

    const rangeStr = part.slice(1);

    if (rangeStr.includes("-")) {
      const rangeParts = rangeStr.split("-");
      const startStr = rangeParts[0];
      const endStr = rangeParts[1];

      if (!startStr || !endStr) {
        toast.error(`Invalid range: "${part}"`);
        continue;
      }

      const start = Number.parseInt(startStr, 10);
      const end = Number.parseInt(endStr, 10);

      if (Number.isNaN(start) || Number.isNaN(end)) {
        toast.error(`Invalid range: "${part}"`);
        continue;
      }

      if (start > end) {
        toast.error(`Invalid range: start (${start}) > end (${end})`);
        continue;
      }

      for (let i = start; i <= end; i++) {
        if (isAdd) add.push(i);
        else remove.push(i);
      }
    } else {
      const id = Number.parseInt(rangeStr, 10);
      if (Number.isNaN(id)) {
        toast.error(`Invalid ID: "${part}"`);
        continue;
      }
      if (isAdd) add.push(id);
      else remove.push(id);
    }
  }

  return { add, remove };
};

const handleParticipantInput = async () => {
  if (!participantInput.value.trim()) return;

  const { add, remove } = parseParticipantInput(participantInput.value);

  // Process removals
  for (const userId of remove) {
    const index = selectedParticipants.value.indexOf(userId);
    if (index !== -1) {
      selectedParticipants.value.splice(index, 1);
    }
  }

  // Process additions
  for (const userId of add) {
    if (selectedParticipants.value.some((p) => p === userId)) {
      continue; // Already added
    }

    try {
      selectedParticipants.value.push(userId);
    } catch (error) {
      toast.error(`Failed to load user ${userId}`);
    }
  }

  participantInput.value = "";
};

const handleRemoveParticipant = (userId: number) => {
  const index = selectedParticipants.value.indexOf(userId);
  if (index !== -1) {
    selectedParticipants.value.splice(index, 1);
  }
};

const loadTrainingPlanData = async (id: string) => {
  isLoading.value = true;
  try {
    const response = await TrainingPlanService.getTrainingPlan(Number(id));

    formData.value = {
      name: response.name,
      description: response.description,
    };

    // Load contests
    for (const contest of response.contests) {
      try {
        const contestDetail = await ContestService.getContest(
          contest.contestId,
          contestPasswordStore.getPassword(contest.contestId),
        );
        selectedContests.value.push({
          contestId: contest.contestId,
          name: contest.name,
          beginTime: contestDetail.beginTime,
          endTime: contestDetail.endTime,
        });
      } catch (e) {
        handleApiError(e);
      }
    }

    // Sort by beginTime
    selectedContests.value.sort(
      (a, b) =>
        new Date(a.beginTime).getTime() - new Date(b.beginTime).getTime(),
    );

    // Load participants
    for (const participant of response.participants) {
      selectedParticipants.value.push(participant.userId);
    }
  } catch (e) {
    handleApiError(e);
  } finally {
    isLoading.value = false;
  }
};

const handleSubmit = async () => {
  if (!formData.value.name.trim()) {
    toast.error("Training plan name is required");
    return;
  }

  if (!formData.value.description.trim()) {
    toast.error("Description is required");
    return;
  }

  isSubmitting.value = true;

  try {
    if (isEditMode.value && planId.value) {
      // Update training plan basic info
      await TrainingPlanService.putTrainingPlan(Number(planId.value), {
        name: formData.value.name,
        description: formData.value.description,
      });

      // Update contests
      await TrainingPlanService.setContests(Number(planId.value), {
        contestIds: selectedContests.value.map((c) => c.contestId),
      });

      // Update participants
      await TrainingPlanService.setParticipants(Number(planId.value), {
        userIds: selectedParticipants.value,
      });

      toast.success("Training plan updated successfully!");
      router.push(buildPath(routeMap.trainingPlan.path, { id: planId.value }));
    } else {
      // Create new training plan
      const response = await TrainingPlanService.createTrainingPlan({
        name: formData.value.name,
        description: formData.value.description,
      });

      const newPlanId = response.planId;

      // Set contests if any
      if (selectedContests.value.length > 0) {
        await TrainingPlanService.setContests(newPlanId, {
          contestIds: selectedContests.value.map((c) => c.contestId),
        });
      }

      // Set participants if any
      if (selectedParticipants.value.length > 0) {
        await TrainingPlanService.setParticipants(newPlanId, {
          userIds: selectedParticipants.value,
        });
      }

      toast.success("Training plan created successfully!");
      router.push(
        buildPath(routeMap.trainingPlan.path, { id: String(newPlanId) }),
      );
    }
  } catch (e) {
    handleApiError(e);
  } finally {
    isSubmitting.value = false;
  }
};

const handleCancel = () => {
  if (isEditMode.value && planId.value) {
    router.push(buildPath(routeMap.trainingPlan.path, { id: planId.value }));
  } else {
    router.push(routeMap.trainingPlanList.path);
  }
};

onMounted(() => {
  const id = route.params.id as string | undefined;
  if (id) {
    isEditMode.value = true;
    planId.value = id;
    loadTrainingPlanData(id);
  }
});
</script>

<template>
  <div class="container mx-auto max-w-6xl">
    <div class="card bg-base-100 shadow-xl">
      <div class="card-body">
        <div v-if="isLoading" class="flex justify-center items-center py-12">
          <span class="loading loading-spinner loading-lg"></span>
        </div>

        <template v-else>
          <h1 class="font-bold card-title">
            {{ isEditMode ? 'Edit Training Plan' : 'Create Training Plan' }}
          </h1>

          <form @submit.prevent="handleSubmit">
            <!-- Training Plan Name -->
            <div class="form-control">
              <label class="label">
                <span class="label-text font-semibold">
                  Training Plan Name
                  <span class="text-error">*</span>
                </span>
              </label>
              <input v-model="formData.name" type="text" placeholder="Enter training plan name"
                class="input input-bordered w-full" required />
            </div>

            <!-- Description -->
            <div class="form-control mt-4">
              <label class="label">
                <span class="label-text font-semibold">
                  Description
                  <span class="text-error">*</span>
                </span>
                <label class="label cursor-pointer gap-2">
                  <span class="label-text-alt">Preview</span>
                  <input type="checkbox" v-model="showDescriptionPreview" class="toggle toggle-sm" />
                </label>
              </label>
              <PreviewableTextEdit v-model="formData.description"
                placeholder="Enter training plan description (Markdown supported)" :rows="15"
                :show-preview="showDescriptionPreview" :required="true" />
              <label class="label">
                <span class="label-text-alt text-base-content/70 inline-flex items-center gap-1">
                  <Icon icon="fa6-solid:circle-info" class="w-4 h-4 inline" />
                  Markdown is supported. You can use code blocks, images, and other Markdown syntax.
                </span>
              </label>
            </div>

            <!-- Contests -->
            <ContestSelector v-model="selectedContests" />

            <!-- Participants -->
            <div class="form-control mt-4">
              <label class="label">
                <span class="label-text font-semibold">
                  Participants ({{ selectedParticipants.length }})
                </span>
              </label>

              <!-- Input for managing participants -->
              <div class="join w-full">
                <input v-model="participantInput" type="text" placeholder="e.g., +1-10, +15, -8-12"
                  class="input input-bordered join-item flex-1" @keydown.enter.prevent="handleParticipantInput" />
                <button type="button" class="btn btn-primary join-item" :disabled="!participantInput.trim()"
                  @click="handleParticipantInput">
                  <Icon icon="fa6-solid:check" width="16" />
                  Apply
                </button>
              </div>

              <label class="label">
                <span class="label-text-alt text-base-content/70 inline-flex items-center gap-1">
                  <Icon icon="fa6-solid:circle-info" class="w-4 h-4 inline" />
                  Use +id to add, -id to remove. Supports ranges: +1-10, -5-8. Multiple operations: +1-5, +10, -3
                </span>
              </label>

              <!-- Selected participants badges -->
              <div v-if="selectedParticipants.length > 0" class="mt-3 flex flex-wrap gap-2">
                <div v-for="participant in selectedParticipants" :key="participant"
                  class="badge badge-neutral badge-lg gap-2 p-3">
                  <span class="font-mono text-xs">{{ participant }}</span>
                  <EntityLink entity-type="user" :entity-id="participant" />
                  <button type="button" class="btn btn-ghost btn-xs btn-circle -ml-1 -mr-2"
                    style="height: 1.25rem; min-height: 1.25rem; width: 1.25rem;"
                    @click.prevent="handleRemoveParticipant(participant)">
                    <Icon icon="fa6-solid:xmark" class="text-xs" />
                  </button>
                </div>

              </div>
            </div>

            <div class="divider"></div>

            <!-- Action Buttons -->
            <div class="flex gap-2 justify-end">
              <button type="button" class="btn" :disabled="isSubmitting" @click="handleCancel">
                <Icon icon="fa6-solid:xmark" width="16" />
                Cancel
              </button>
              <button type="submit" class="btn btn-primary" :disabled="isSubmitting">
                <span v-if="isSubmitting" class="loading loading-spinner loading-sm"></span>
                <Icon v-else icon="fa6-solid:check" width="16" />
                {{ isEditMode ? 'Update Training Plan' : 'Create Training Plan' }}
              </button>
            </div>
          </form>
        </template>
      </div>
    </div>
  </div>
</template>
