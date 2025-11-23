<script setup lang="ts">
import { Icon } from "@iconify/vue";
import { nextTick, onMounted, type Ref, ref, watch } from "vue";
import { useRoute, useRouter } from "vue-router";
import { useToast } from "vue-toastification";
import InputModal from "@/components/Modal/modals/InputModal.vue";
import { useModal } from "@/components/Modal/useModal.mjs";
import PreviewableTextEdit from "@/components/PreviewableTextEdit.vue";
import { useApiErrorHandler } from "@/composables/useApiErrorHandler.mjs";
import { buildPath, routeMap } from "@/routes.mjs";
import { useContestPasswordStore } from "@/stores/contestPassword.mts";

// biome-ignore lint/style/useImportType: <ContestStatus is used as enum values in the template section>
import {
  ContestService,
  ContestStatus,
  ContestType,
  type CreateContestRequest,
  ProblemService,
  type UpdateContestRequest,
} from "@/theoj-api";

const { handleApiError } = useApiErrorHandler();
const router = useRouter();
const route = useRoute();
const toast = useToast();

const MAX_PROBLEMS = 10;

const isSubmitting = ref(false);
const isLoading = ref(false);
const showDescriptionPreview = ref(false);
const isEditMode = ref(false);
const contestId = ref<string | null>(null);

const formData = ref<CreateContestRequest & { status?: ContestStatus | null }>({
  name: "",
  description: "",
  type: ContestType.PUBLIC,
  beginTime: "",
  endTime: "",
  password: null,
  problemIds: [],
  status: undefined,
});

const problemIdInput: Ref<number | null> = ref(null);
const selectedProblems = ref<Array<{ id: number; name: string }>>([]);
const isLoadingProblem = ref(false);
const draggedIndex = ref<number | null>(null);
const previewProblem = ref<{ id: number; name: string } | null>(null);
const problemIdInputRef = ref<HTMLInputElement | null>(null);

const contestPasswordStore = useContestPasswordStore();
const promptForPassword = (isWrongPassword = false) => {
  const { open, close } = useModal({
    component: InputModal,
    attrs: {
      title: "Contest Password Required",
      placeholder: "Enter contest password",
      inputType: "password",
      confirmText: "Submit",
      cancelText: "Cancel",
      errorMessage: isWrongPassword ? "Incorrect password. Please try again." : "",
      initialValue: contestId.value ? contestPasswordStore.getPassword(Number(contestId.value)) || "" : "",
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


const getLabel = (index: number): string => {
  return String.fromCharCode(65 + index); // 65 is 'A'
};

watch(problemIdInput, async (newId) => {
  if (!newId || selectedProblems.value.some((p) => p.id === newId)) {
    previewProblem.value = null;
    return;
  }
  isLoadingProblem.value = true;
  const wasFocused = document.activeElement === problemIdInputRef.value;
  try {
    const response = await ProblemService.getProblem(String(newId));
    previewProblem.value = {
      id: Number(response.problemId),
      name: response.name,
    };
  } catch (error) {
    previewProblem.value = null;
  } finally {
    isLoadingProblem.value = false;
    if (wasFocused) {
      nextTick(() => problemIdInputRef.value?.focus());
    }
  }
});

const handleAddProblem = async (event?: Event) => {
  event?.preventDefault();
  if (!problemIdInput.value || selectedProblems.value.length >= MAX_PROBLEMS) {
    return;
  }
  // if already exists
  if (selectedProblems.value.some((p) => p.id === problemIdInput.value)) {
    return;
  }
  // use preview data
  if (
    previewProblem.value &&
    previewProblem.value.id === problemIdInput.value
  ) {
    selectedProblems.value.push(previewProblem.value);
    problemIdInput.value = null;
    previewProblem.value = null;
    return;
  }
  isLoadingProblem.value = true;
  try {
    const response = await ProblemService.getProblem(
      String(problemIdInput.value),
    );
    selectedProblems.value.push({
      id: Number(response.problemId),
      name: response.name,
    });
    problemIdInput.value = null;
    previewProblem.value = null;
  } catch (error) {
    handleApiError(error);
  } finally {
    isLoadingProblem.value = false;
  }
};

const handleRemoveProblem = (index: number) => {
  selectedProblems.value.splice(index, 1);
};

const handleDragStart = (index: number) => {
  draggedIndex.value = index;
};

const handleDragOver = (event: DragEvent) => {
  event.preventDefault();
};

const handleDrop = (event: DragEvent, dropIndex: number) => {
  event.preventDefault();
  if (draggedIndex.value === null) return;
  const draggedItem = selectedProblems.value[draggedIndex.value];
  if (!draggedItem) return;
  selectedProblems.value.splice(draggedIndex.value, 1);
  selectedProblems.value.splice(dropIndex, 0, draggedItem);
  draggedIndex.value = null;
};

const handleDragEnd = () => {
  draggedIndex.value = null;
};

const loadContestData = async (id: string, password?: string) => {
  isLoading.value = true;
  try {
    const storedPassword = password ?? contestPasswordStore.getPassword(Number(id));
    const response = await ContestService.getContest(id, storedPassword ?? undefined);
    // If success and password was used, save it
    if (storedPassword) {
      contestPasswordStore.setPassword(Number(id), storedPassword);
    }
    formData.value = {
      name: response.name,
      description: response.description,
      type: response.type,
      beginTime: new Date(response.beginTime).toISOString().slice(0, 16),
      endTime: new Date(response.endTime).toISOString().slice(0, 16),
      password: null,
      problemIds: response.problemIds,
      status: response.status,
    };
    // Load problem details
    for (const problemId of response.problemIds) {
      try {
        const problem = await ProblemService.getProblem(problemId.toString());
        selectedProblems.value.push({
          id: problemId,
          name: problem.name,
        });
      } catch (e) {
        console.error(`Failed to load problem ${problemId}:`, e);
        selectedProblems.value.push({
          id: problemId,
          name: `Problem ${problemId}`,
        });
      }
    }
  } catch (e: unknown) {
    const err = e as { status?: number; body?: string | { message?: string } };

    // Check if it's a password-related error
    if (err.status === 403) {
      const body = typeof err.body === "string" ? JSON.parse(err.body) : err.body;
      const message = body?.message || "";

      if (message === "contest password required") {
        isLoading.value = false;
        promptForPassword(false);
        return;
      }

      if (message === "contest password wrong") {
        isLoading.value = false;
        // Clear wrong password
        if (contestId.value) {
          contestPasswordStore.clearPassword(Number(contestId.value));
        }
        promptForPassword(true);
        return;
      }
    }

    handleApiError(e);
  } finally {
    isLoading.value = false;
  }
};

const handleSubmit = async () => {
  if (!formData.value.name.trim()) {
    toast.error("Contest name is required");
    return;
  }

  if (!formData.value.description.trim()) {
    toast.error("Description is required");
    return;
  }

  if (!formData.value.beginTime) {
    toast.error("Begin time is required");
    return;
  }

  if (!formData.value.endTime) {
    toast.error("End time is required");
    return;
  }

  const beginDate = new Date(formData.value.beginTime);
  const endDate = new Date(formData.value.endTime);

  if (endDate <= beginDate) {
    toast.error("End time must be after begin time");
    return;
  }

  if (selectedProblems.value.length === 0) {
    toast.error("At least one problem is required");
    return;
  }

  if (selectedProblems.value.length > MAX_PROBLEMS) {
    toast.error(`Maximum ${MAX_PROBLEMS} problems allowed per contest`);
    return;
  }

  isSubmitting.value = true;

  try {
    if (isEditMode.value && contestId.value) {
      const requestData: UpdateContestRequest = {
        name: formData.value.name,
        description: formData.value.description,
        type: formData.value.type,
        beginTime: new Date(formData.value.beginTime).toISOString(),
        endTime: new Date(formData.value.endTime).toISOString(),
        password: formData.value.password?.trim()
          ? formData.value.password
          : null,
        problemIds: selectedProblems.value.map((p) => p.id),
        status: formData.value.status ?? null,
      };

      await ContestService.putContest(contestId.value, requestData);
      if (requestData.password) {
        contestPasswordStore.setPassword(Number(contestId.value), requestData.password);
      }
      toast.success("Contest updated successfully!");
      // router.push(buildPath(routeMap.contest.path, { id: contestId.value }));
    } else {
      const requestData: CreateContestRequest = {
        name: formData.value.name,
        description: formData.value.description,
        type: formData.value.type,
        beginTime: new Date(formData.value.beginTime).toISOString(),
        endTime: new Date(formData.value.endTime).toISOString(),
        password: formData.value.password?.trim()
          ? formData.value.password
          : null,
        problemIds: selectedProblems.value.map((p) => p.id),
      };

      const response = await ContestService.createContest(requestData);
      if (requestData.password) {
        contestPasswordStore.setPassword(Number(response.contestId), requestData.password);
      }
      toast.success("Contest created successfully!");
      // router.push(buildPath(routeMap.contest.path, { id: response.contestId }));
    }
  } catch (e) {
    handleApiError(e);
  } finally {
    isSubmitting.value = false;
  }
};

const handleCancel = () => {
  if (isEditMode.value && contestId.value) {
    router.push(buildPath(routeMap.contestList.path, { id: contestId.value }));
  } else {
    router.push(routeMap.contestList.path);
  }
};

onMounted(() => {
  const id = route.params.id as string | undefined;
  if (id) {
    isEditMode.value = true;
    contestId.value = id;
    loadContestData(id);
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
            {{ isEditMode ? 'Edit Contest' : 'Create Contest' }}
          </h2>

          <form @submit.prevent="handleSubmit">
            <!-- Contest Name -->
            <div class="form-control">
              <label class="label">
                <span class="label-text font-semibold">
                  Contest Name
                  <span class="text-error">*</span>
                </span>
              </label>
              <input v-model="formData.name" type="text" placeholder="Enter contest name"
                class="input input-bordered w-full" required />
            </div>

            <!-- Contest Type -->
            <div class="form-control mt-4">
              <label class="label">
                <span class="label-text font-semibold">
                  Contest Type
                  <span class="text-error">*</span>
                </span>
              </label>
              <select v-model="formData.type" class="select select-bordered w-full">
                <option :value="ContestType.PUBLIC">Public</option>
                <option :value="ContestType.PRIVATE">Private</option>
              </select>
            </div>

            <!-- Contest Status (Edit mode only) -->
            <div v-if="isEditMode" class="form-control mt-4">
              <label class="label">
                <span class="label-text font-semibold">Contest Status</span>
              </label>
              <select v-model="formData.status" class="select select-bordered w-full">
                <option :value="ContestStatus.ACTIVE">Active</option>
                <option :value="ContestStatus.HIDDEN">Hidden</option>
              </select>
            </div>

            <!-- Begin Time -->
            <div class="form-control mt-4">
              <label class="label">
                <span class="label-text font-semibold">
                  Begin Time
                  <span class="text-error">*</span>
                </span>
              </label>
              <input v-model="formData.beginTime" type="datetime-local" class="input input-bordered w-full" required />
            </div>

            <!-- End Time -->
            <div class="form-control mt-4">
              <label class="label">
                <span class="label-text font-semibold">
                  End Time
                  <span class="text-error">*</span>
                </span>
              </label>
              <input v-model="formData.endTime" type="datetime-local" class="input input-bordered w-full" required />
            </div>

            <!-- Password -->
            <div class="form-control mt-4">
              <label class="label">
                <span class="label-text font-semibold">
                  Password {{ isEditMode ? '(Leave empty to keep current)' : '(Optional)' }}
                </span>
              </label>
              <input v-model="formData.password" type="password"
                :placeholder="isEditMode ? 'Enter new password or leave empty' : 'Leave empty for no password'"
                class="input input-bordered w-full" />
              <label class="label">
                <span class="label-text-alt text-base-content/70 inline-flex items-center gap-1">
                  <Icon icon="fa6-solid:circle-info" class="w-4 h-4 inline" />
                  Set a password to restrict access to this contest
                </span>
              </label>
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
                placeholder="Enter contest description (Markdown supported)" :rows="20"
                :show-preview="showDescriptionPreview" :required="true" />
              <label class="label">
                <span class="label-text-alt text-base-content/70 inline-flex items-center gap-1">
                  <Icon icon="fa6-solid:circle-info" class="w-4 h-4 inline" />
                  Markdown is supported. You can use code blocks, images, and other Markdown syntax.
                </span>
              </label>
            </div>

            <!-- Problem IDs -->
            <div class="form-control mt-4">
              <label class="label">
                <span class="label-text font-semibold">
                  Problems ({{ selectedProblems.length }}/{{ MAX_PROBLEMS }})
                  <span class="text-error">*</span>
                </span>
              </label>
              <!-- Input for adding problems -->
              <div class="join w-full">
                <input ref="problemIdInputRef" v-model.number="problemIdInput" type="number"
                  placeholder="Enter problem ID" class="input input-bordered join-item flex-1"
                  :disabled="isLoadingProblem || selectedProblems.length >= MAX_PROBLEMS"
                  @keydown.enter.prevent="handleAddProblem" />
                <button type="button" class="btn btn-primary join-item"
                  :disabled="isLoadingProblem || selectedProblems.length >= MAX_PROBLEMS || !problemIdInput"
                  @click="handleAddProblem">
                  <span v-if="isLoadingProblem" class="loading loading-spinner loading-sm"></span>
                  <Icon v-else icon="fa6-solid:plus" width="16" />
                  Add
                </button>
              </div>
              <!-- Selected problems list -->
              <div v-if="selectedProblems.length > 0 || previewProblem" class="mt-3 space-y-2">
                <!-- Existing problems -->
                <div v-for="(problem, index) in selectedProblems" :key="problem.id" @dragover="handleDragOver"
                  @drop="handleDrop($event, index)"
                  class="flex items-center h-14 gap-2 p-3 bg-base-200 rounded-lg border-2 border-transparent hover:bg-base-300 transition-colors">
                  <div class="cursor-move flex items-center" draggable="true" @dragstart="handleDragStart(index)"
                    @dragend="handleDragEnd">
                    <Icon icon="fa6-solid:grip-vertical" width="16" class="text-base-content/50 mx-1.5" />
                  </div>
                  <div class="flex items-center gap-2 flex-1">
                    <span class="badge badge-primary">{{ getLabel(index) }}</span>
                    <span class="badge badge-neutral">{{ problem.id }}</span>
                    <RouterLink :to="buildPath(routeMap.problem.path, { id: problem.id })" class="flex-1">
                      {{ problem.name }}
                    </RouterLink>
                  </div>
                  <button type="button" class="btn btn-ghost btn-sm btn-circle"
                    @click.prevent="handleRemoveProblem(index)">
                    <Icon icon="fa6-solid:xmark" width="16" />
                  </button>
                </div>

                <!-- Preview problem (not yet added) -->
                <div v-if="previewProblem" :to="buildPath(routeMap.problem.path, { id: previewProblem.id })"
                  class="flex items-center h-14 gap-2 p-3 bg-base-200/50 rounded-lg border-2 border-dashed border-base-300 hover:bg-base-200 transition-colors opacity-60">
                  <Icon icon="fa6-solid:grip-vertical" width="16" class="text-base-content/30 mx-1.5 invisible" />
                  <span class="badge badge-outline">{{ getLabel(selectedProblems.length) }}</span>
                  <span class="badge badge-ghost badge-outline">{{ previewProblem.id }}</span>
                  <RouterLink :to="buildPath(routeMap.problem.path, { id: previewProblem.id })" class="flex-1 italic">
                    {{ previewProblem.name }}
                  </RouterLink>
                  <Icon icon="fa6-solid:eye" width="16" class="text-base-content/50 mx-1.5" />
                </div>
              </div>
              <label class="label">
                <span class="label-text-alt text-base-content/70 inline-flex items-center gap-1">
                  <Icon icon="fa6-solid:circle-info" class="w-4 h-4 inline" />
                  Enter problem ID to preview. Click Add to confirm. Drag and drop to reorder. Maximum {{ MAX_PROBLEMS
                  }} problems.
                </span>
              </label>
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
                {{ isEditMode ? 'Update Contest' : 'Create Contest' }}
              </button>
            </div>
          </form>
        </template>
      </div>
    </div>
  </div>
</template>
