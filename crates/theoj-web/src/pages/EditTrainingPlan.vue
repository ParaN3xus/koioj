<script setup lang="ts">
import { Icon } from "@iconify/vue";
import { nextTick, onMounted, type Ref, ref, watch } from "vue";
import { useRoute, useRouter } from "vue-router";
import { useToast } from "vue-toastification";
import EntityLink from "@/components/EntityLink.vue";
import PreviewableTextEdit from "@/components/PreviewableTextEdit.vue";
import { useApiErrorHandler } from "@/composables/useApiErrorHandler.mjs";
import { useContestPasswordPrompt } from "@/composables/useContestPasswordPrompt.mjs";
import { buildPath, routeMap } from "@/routes.mjs";
import { useContestPasswordStore } from "@/stores/contestPassword.mjs";
import {
  ContestService,
  type CreateTrainingPlanRequest,
  TrainingPlanService,
} from "@/theoj-api";
import { formatDateTime } from "@/utils.mjs";

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

// Contest management
const contestIdInput: Ref<number | null> = ref(null);
interface ContestInfo {
  id: number;
  name: string;
  beginTime: string;
  endTime: string;
}
const selectedContests = ref<Array<ContestInfo>>([]);
const isLoadingContest = ref(false);
const previewContest = ref<ContestInfo | null>(null);
const contestIdInputRef = ref<HTMLInputElement | null>(null);
const contestPasswordStore = useContestPasswordStore();

// Participant management
const participantInput = ref("");
const selectedParticipants = ref<Array<number>>([]);

const getLabel = (index: number): string => {
  return String.fromCharCode(65 + index); // 65 is 'A'
};

// Watch contest input for preview
watch(contestIdInput, async (newId) => {
  if (!newId || selectedContests.value.some((c) => c.id === newId)) {
    previewContest.value = null;
    return;
  }
  isLoadingContest.value = true;
  const wasFocused = document.activeElement === contestIdInputRef.value;

  try {
    const response = await ContestService.getContest(
      newId,
      contestPasswordStore.getPassword(newId),
    );
    previewContest.value = {
      id: Number(response.contestId),
      name: response.name,
      beginTime: response.beginTime,
      endTime: response.endTime,
    };
  } catch (error) {
    previewContest.value = null;
  } finally {
    isLoadingContest.value = false;
    if (wasFocused) {
      nextTick(() => contestIdInputRef.value?.focus());
    }
  }
});

const handleAddContest = async (event?: Event) => {
  event?.preventDefault();
  if (!contestIdInput.value) {
    return;
  }
  // if already exists
  if (selectedContests.value.some((c) => c.id === contestIdInput.value)) {
    return;
  }
  // use preview data
  if (
    previewContest.value &&
    previewContest.value.id === contestIdInput.value
  ) {
    selectedContests.value.push(previewContest.value);
    contestIdInput.value = null;
    previewContest.value = null;
    return;
  }

  await addContestById(contestIdInput.value);
};

const addContestById = async (id: number, password?: string) => {
  try {
    isLoadingContest.value = true;

    const storedPassword =
      password || contestPasswordStore.getPassword(Number(id));

    const response = await ContestService.getContest(
      id,
      storedPassword || null,
    );

    selectedContests.value.push({
      id: Number(response.contestId),
      name: response.name,
      beginTime: response.beginTime,
      endTime: response.endTime,
    });

    if (storedPassword && !password) {
      contestPasswordStore.setPassword(Number(id), storedPassword);
    } else if (password) {
      contestPasswordStore.setPassword(Number(id), password);
    }

    contestIdInput.value = null;
    previewContest.value = null;
  } catch (e: unknown) {
    const err = e as { status?: number; body?: string | { message?: string } };

    if (err.status === 403) {
      const body =
        typeof err.body === "string" ? JSON.parse(err.body) : err.body;
      const message = body?.message || "";

      if (message === "contest password required") {
        isLoadingContest.value = false;
        const { promptForPassword } = useContestPasswordPrompt({
          contestId: Number(id),
          onPasswordSubmit: async (pwd: string) => {
            await addContestById(id, pwd);
          },
        });
        promptForPassword(false);
        return;
      }

      if (message === "contest password wrong") {
        isLoadingContest.value = false;
        contestPasswordStore.clearPassword(Number(id));
        const { promptForPassword } = useContestPasswordPrompt({
          contestId: Number(id),
          onPasswordSubmit: async (pwd: string) => {
            await addContestById(id, pwd);
          },
        });
        promptForPassword(true);
        return;
      }
    }

    handleApiError(e);
  } finally {
    isLoadingContest.value = false;
  }
};

const handleRemoveContest = (index: number) => {
  selectedContests.value.splice(index, 1);
};

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
          id: contest.contestId,
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
        contestIds: selectedContests.value.map((c) => c.id),
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
          contestIds: selectedContests.value.map((c) => c.id),
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
          <h2 class="card-title mb-4">
            {{ isEditMode ? 'Edit Training Plan' : 'Create Training Plan' }}
          </h2>

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
            <div class="form-control mt-4">
              <label class="label">
                <span class="label-text font-semibold">
                  Contests ({{ selectedContests.length }})
                </span>
              </label>
              <!-- Input for adding contests -->
              <div class="join w-full">
                <input ref="contestIdInputRef" v-model.number="contestIdInput" type="number"
                  placeholder="Enter contest ID" class="input input-bordered join-item flex-1"
                  :disabled="isLoadingContest" @keydown.enter.prevent="handleAddContest" />
                <button type="button" class="btn btn-primary join-item" :disabled="isLoadingContest || !contestIdInput"
                  @click="handleAddContest">
                  <span v-if="isLoadingContest" class="loading loading-spinner loading-sm"></span>
                  <Icon v-else icon="fa6-solid:plus" width="16" />
                  Add
                </button>
              </div>
              <!-- Selected contests table -->
              <div v-if="selectedContests.length > 0 || previewContest" class="space-y-2">
                <div class="overflow-x-auto">
                  <table class="table table-zebra w-full">
                    <thead>
                      <tr>
                        <th>Contest ID</th>
                        <th>Contest Name</th>
                        <th>Begin Time</th>
                        <th>End Time</th>
                        <th></th>
                      </tr>
                    </thead>
                    <tbody>
                      <tr v-for="(contest, index) in selectedContests" :key="contest.id">
                        <td>
                          {{ contest.id }}
                        </td>
                        <td>
                          <EntityLink entity-type="contest" :entity-id="contest.id">
                            {{ contest.name }}
                          </EntityLink>
                        </td>
                        <td>{{ formatDateTime(contest.beginTime) }}</td>
                        <td>{{ formatDateTime(contest.endTime) }}</td>
                        <td class="text-right">
                          <button type="button" class="btn btn-ghost btn-sm btn-circle"
                            @click="handleRemoveContest(index)">
                            <Icon icon="fa6-solid:xmark" class="text-lg" />
                          </button>
                        </td>
                      </tr>
                      <tr v-if="previewContest" class="opacity-50">
                        <td>
                          {{ previewContest.id }}
                        </td>
                        <td>
                          <EntityLink entity-type="contest" :entity-id="previewContest.id">
                            {{ previewContest.name }}
                          </EntityLink>
                        </td>
                        <td>{{ formatDateTime(previewContest.beginTime) }}</td>
                        <td>{{ formatDateTime(previewContest.endTime) }}</td>
                        <td class="text-right flex justify-end">
                          <div class="h-8 w-8 flex items-center justify-center">
                            <Icon icon="fa6-solid:eye" class="text-lg" />
                          </div>
                        </td>
                      </tr>
                    </tbody>
                  </table>
                </div>
              </div>

              <label class="label">
                <span class="label-text-alt text-base-content/70 inline-flex items-center gap-1">
                  <Icon icon="fa6-solid:circle-info" class="w-4 h-4 inline" />
                  Enter contest ID to preview. Click Add to confirm.
                </span>
              </label>
            </div>

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
                <div v-for="participant in selectedParticipants" class="badge badge-neutral badge-lg gap-2 p-3">
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
