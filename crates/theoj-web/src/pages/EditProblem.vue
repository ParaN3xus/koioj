<script setup lang="ts">
import { Icon } from "@iconify/vue";
import JSZip from "jszip";
import { computed, onMounted, ref } from "vue";
import { useRoute, useRouter } from "vue-router";
import { useToast } from "vue-toastification";
import PreviewableTextEdit from "@/components/PreviewableTextEdit.vue";
import TestCaseEditor from "@/components/TestCaseEditor.vue";
import { useApiErrorHandler } from "@/composables/useApiErrorHandler.mjs";
import { buildPath, routeMap } from "@/routes.mjs";
import {
  type CreateProblemRequest,
  type GetProblemResponse,
  ProblemService,
  ProblemStatus,
  type PutProblemRequest,
  type TestCaseData,
  UserRole,
  UserService,
} from "@/theoj-api";
import { useUserStore } from "@/user.mjs";

const { handleApiError } = useApiErrorHandler();
const router = useRouter();
const route = useRoute();
const toast = useToast();
const userStore = useUserStore();

const problemId = computed(() => route.params.id as string | undefined);
const isEditMode = computed(() => !!problemId.value);
const pageTitle = computed(() =>
  isEditMode.value ? "Edit Problem" : "Create Problem",
);

const currentUserRole = ref<UserRole | null>(null);
const isLoading = ref(true);
const isSaving = ref(false);

// Form data
const formData = ref<GetProblemResponse & { status: ProblemStatus }>({
  problemId: "",
  name: "",
  description: "",
  inputDescription: "",
  outputDescription: "",
  note: "",
  timeLimit: 1000,
  memLimit: 256,
  status: ProblemStatus.HIDDEN,
  samples: [],
});

// Test cases
const samples = ref<TestCaseData[]>([]);
const existingTestCases = ref<number[]>([]);
const newTestCases = ref<Array<TestCaseData & { fromZip?: boolean }>>([]);

// Preview toggles
const showDescriptionPreview = ref(false);
const showInputDescPreview = ref(false);
const showOutputDescPreview = ref(false);
const showNotePreview = ref(false);

const canEdit = computed(() => {
  return (
    currentUserRole.value === UserRole.ADMIN ||
    currentUserRole.value === UserRole.TEACHER
  );
});

const loadProblemData = async () => {
  if (!isEditMode.value) {
    isLoading.value = false;
    return;
  }

  const id = problemId.value;
  if (!id) {
    toast.error("Invalid problem ID");
    router.push(routeMap.index.path);
    return;
  }

  isLoading.value = true;
  try {
    const roleResponse = await UserService.getRole(userStore.userId);
    currentUserRole.value = roleResponse.role;

    if (!canEdit.value) {
      toast.error("You don't have permission to edit problems");
      router.push(routeMap.index.path);
      return;
    }

    const problemResponse = await ProblemService.getProblem(id);
    formData.value = {
      ...problemResponse,
      status: ProblemStatus.HIDDEN,
    };

    samples.value = problemResponse.samples || [];

    const testCasesResponse = await ProblemService.getTestCases(id);
    existingTestCases.value = testCasesResponse.testCases;

    toast.success("Problem loaded!");
  } catch (e) {
    handleApiError(e);
  } finally {
    isLoading.value = false;
  }
};

onMounted(async () => {
  const role = (await UserService.getRole(userStore.userId)).role;
  if (!(role === UserRole.ADMIN || role === UserRole.TEACHER)) {
    toast.error(`Permission denied!`);
    router.push(routeMap.index.path);
  }

  loadProblemData();
});

// Sample management
const addSample = () => {
  samples.value.push({
    input: "",
    output: "",
  });
};

const removeSample = (index: number) => {
  samples.value.splice(index, 1);
};

// Test case file upload
const fileInput = ref<HTMLInputElement | null>(null);
const isProcessingZip = ref(false);

const handleFileSelect = () => {
  fileInput.value?.click();
};

const handleFileChange = async (event: Event) => {
  const target = event.target as HTMLInputElement;
  const file = target.files?.[0];

  if (!file) return;

  if (!file.name.endsWith(".zip")) {
    toast.error("Please upload a ZIP file");
    return;
  }

  isProcessingZip.value = true;
  try {
    const zip = await JSZip.loadAsync(file);

    const testCases: Array<TestCaseData & { fromZip: boolean }> = [];
    const folders = new Set<string>();

    Object.keys(zip.files).forEach((filename) => {
      const parts = filename.split("/");
      if (parts.length > 1 && parts[0]) {
        folders.add(parts[0]);
      }
    });

    for (const folder of folders) {
      const inFile = zip.files[`${folder}/in.txt`];
      const outFile = zip.files[`${folder}/out.txt`];
      if (inFile && outFile && !inFile.dir && !outFile.dir) {
        const input = await inFile.async("text");
        const output = await outFile.async("text");
        testCases.push({ input, output, fromZip: true });
      }
    }
    if (testCases.length === 0) {
      toast.error(
        "No valid test cases found. Expected folders with in.txt and out.txt",
      );
      return;
    }
    newTestCases.value.push(...testCases);
    toast.success(`Imported ${testCases.length} test cases`);
  } catch (e) {
    toast.error("Failed to process ZIP file");
    console.error(e);
  } finally {
    isProcessingZip.value = false;
    if (target) {
      target.value = "";
    }
  }
};

// Manual test case management
const addManualTestCase = () => {
  newTestCases.value.push({
    input: "",
    output: "",
    fromZip: false,
  });
};

const removeNewTestCase = (index: number) => {
  newTestCases.value.splice(index, 1);
};

// Save problem
const handleSave = async () => {
  if (!formData.value.name.trim()) {
    toast.error("Problem name is required");
    return;
  }

  if (!formData.value.description.trim()) {
    toast.error("Problem description is required");
    return;
  }

  if (!formData.value.inputDescription.trim()) {
    toast.error("Input description is required");
    return;
  }

  if (!formData.value.outputDescription.trim()) {
    toast.error("Output description is required");
    return;
  }

  isSaving.value = true;
  try {
    if (isEditMode.value) {
      const id = problemId.value;
      if (!id) {
        toast.error("Invalid problem ID");
        return;
      }

      // Update existing problem
      const updateRequest: PutProblemRequest = {
        name: formData.value.name,
        description: formData.value.description,
        inputDescription: formData.value.inputDescription,
        outputDescription: formData.value.outputDescription,
        note: formData.value.note || null,
        timeLimit: formData.value.timeLimit,
        memLimit: formData.value.memLimit,
        status: formData.value.status,
        samples: samples.value.length > 0 ? samples.value : null,
      };

      await ProblemService.putProblem(id, updateRequest);

      // Add new test cases if any
      if (newTestCases.value.length > 0) {
        await ProblemService.addTestCases(id, {
          testCases: newTestCases.value,
        });
      }

      toast.success("Problem updated successfully!");
    } else {
      // Create new problem
      const createRequest: CreateProblemRequest = {
        name: formData.value.name,
        description: formData.value.description,
        inputDescription: formData.value.inputDescription,
        outputDescription: formData.value.outputDescription,
        note: formData.value.note || null,
        timeLimit: formData.value.timeLimit,
        memLimit: formData.value.memLimit,
        status: formData.value.status,
        samples: samples.value,
      };

      const response = await ProblemService.createProblem(createRequest);

      // Add test cases if any
      if (newTestCases.value.length > 0) {
        await ProblemService.addTestCases(response.problemId, {
          testCases: newTestCases.value,
        });
      }

      toast.success("Problem created successfully!");
      router.push(buildPath(routeMap.problem.path, { id: response.problemId }));
    }
  } catch (e) {
    handleApiError(e);
  } finally {
    isSaving.value = false;
  }
};

const handleCancel = () => {
  router.back();
};
</script>

<template>
  <div class="container mx-auto max-w-6xl">
    <div class="card bg-base-100 shadow-xl">
      <div class="card-body">
        <!-- Header -->
        <div class="flex items-center justify-between mb-4">
          <h2 class="card-title text-2xl">
            {{ pageTitle }}
          </h2>
        </div>

        <!-- Loading State -->
        <div v-if="isLoading" class="flex items-center justify-center py-8">
          <span class="loading loading-spinner loading-lg"></span>
        </div>

        <!-- Form -->
        <div v-else class="space-y-6">
          <!-- Basic Information -->
          <div class="space-y-4">
            <h3 class="text-lg font-semibold flex items-center gap-2">
              <Icon icon="fa7-solid:circle-info" width="20" />
              Basic Information
            </h3>

            <div class="form-control">
              <label class="label">
                <span class="label-text">Problem Name *</span>
              </label>
              <input v-model="formData.name" type="text" placeholder="Enter problem name" class="input input-bordered"
                required />
            </div>

            <div class="form-control">
              <label class="label">
                <span class="label-text">Status *</span>
              </label>
              <select v-model="formData.status" class="select select-bordered" required>
                <option :value="ProblemStatus.HIDDEN">Hidden</option>
                <option :value="ProblemStatus.ACTIVE">Active</option>
              </select>
            </div>

            <div class="grid grid-cols-2 gap-4">
              <div class="form-control">
                <label class="label">
                  <span class="label-text">Time Limit (ms) *</span>
                </label>
                <input v-model.number="formData.timeLimit" type="number" min="1" placeholder="1000"
                  class="input input-bordered" required />
              </div>

              <div class="form-control">
                <label class="label">
                  <span class="label-text">Memory Limit (MB) *</span>
                </label>
                <input v-model.number="formData.memLimit" type="number" min="1" placeholder="256"
                  class="input input-bordered" required />
              </div>
            </div>

            <!-- Description with Preview -->
            <div class="form-control">
              <label class="label">
                <span class="label-text">Description *</span>
                <label class="label cursor-pointer gap-2">
                  <span class="label-text-alt">Preview</span>
                  <input type="checkbox" v-model="showDescriptionPreview" class="toggle toggle-sm" />
                </label>
              </label>
              <PreviewableTextEdit v-model="formData.description"
                placeholder="Enter problem description (Markdown supported)" :rows="8"
                :show-preview="showDescriptionPreview" :required="true" />
            </div>

            <!-- Input Description with Preview -->
            <div class="form-control">
              <label class="label">
                <span class="label-text">Input Description *</span>
                <label class="label cursor-pointer gap-2">
                  <span class="label-text-alt">Preview</span>
                  <input type="checkbox" v-model="showInputDescPreview" class="toggle toggle-sm" />
                </label>
              </label>
              <PreviewableTextEdit v-model="formData.inputDescription"
                placeholder="Describe input format (Markdown supported)" :rows="6" :show-preview="showInputDescPreview"
                :required="true" />
            </div>

            <!-- Output Description with Preview -->
            <div class="form-control">
              <label class="label">
                <span class="label-text">Output Description *</span>
                <label class="label cursor-pointer gap-2">
                  <span class="label-text-alt">Preview</span>
                  <input type="checkbox" v-model="showOutputDescPreview" class="toggle toggle-sm" />
                </label>
              </label>
              <PreviewableTextEdit v-model="formData.outputDescription"
                placeholder="Describe output format (Markdown supported)" :rows="6"
                :show-preview="showOutputDescPreview" :required="true" />
            </div>

            <!-- Note with Preview -->
            <div class="form-control">
              <label class="label">
                <span class="label-text">Note</span>
                <label class="label cursor-pointer gap-2">
                  <span class="label-text-alt">Preview</span>
                  <input type="checkbox" v-model="showNotePreview" class="toggle toggle-sm" />
                </label>
              </label>
              <PreviewableTextEdit v-model="formData.note!"
                placeholder="Additional notes (optional, Markdown supported)" :rows="6"
                :show-preview="showNotePreview" />
            </div>
          </div>

          <div class="divider"></div>

          <!-- Sample Cases -->
          <div class="space-y-4">
            <div class="flex items-center justify-between">
              <h3 class="text-lg font-semibold flex items-center gap-2">
                <Icon icon="fa7-solid:flask" width="20" />
                Sample Cases
              </h3>
              <button class="btn btn-sm btn-primary" @click="addSample">
                <Icon icon="fa7-solid:plus" width="14" />
                Add Sample
              </button>
            </div>
            <div v-if="samples.length === 0" class="text-center py-8 text-base-content/70">
              <Icon icon="fa7-solid:inbox" width="32" class="mx-auto mb-2" />
              <p>No sample cases yet</p>
            </div>
            <div v-else class="space-y-4">
              <TestCaseEditor v-for="(sample, index) in samples" :key="index" :test-case="sample" :index="index"
                :title="`Sample ${index + 1}`" :removable="true" @remove="removeSample(index)" />
            </div>
          </div>

          <div class="divider"></div>

          <!-- Test Cases -->
          <div class="space-y-4">
            <div class="flex items-center justify-between">
              <h3 class="text-lg font-semibold flex items-center gap-2">
                <Icon icon="fa7-solid:vial" width="20" />
                Test Cases
              </h3>
              <div class="flex gap-2">
                <button class="btn btn-sm" @click="handleFileSelect" :disabled="isProcessingZip">
                  <Icon v-if="!isProcessingZip" icon="fa7-solid:file-zipper" width="14" />
                  <span v-else class="loading loading-spinner loading-xs"></span>
                  Import ZIP
                </button>
                <button class="btn btn-sm btn-primary" @click="addManualTestCase">
                  <Icon icon="fa7-solid:plus" width="14" />
                  Add Manually
                </button>
              </div>
            </div>

            <input ref="fileInput" type="file" accept=".zip" class="hidden" @change="handleFileChange" />

            <!-- Existing Test Cases (Edit mode only) -->
            <div v-if="isEditMode && existingTestCases.length > 0">
              <h4 class="text-sm font-semibold mb-2 text-base-content/70">
                Existing Test Cases ({{ existingTestCases.length }})
              </h4>
              <div class="flex flex-wrap gap-4">
                <div v-for="testCase in existingTestCases" :key="testCase" class="badge badge-lg badge-neutral gap-2">
                  <Icon icon="fa7-solid:check" width="12" />
                  Test {{ testCase }}
                </div>
              </div>
            </div>

            <!-- New Test Cases -->
            <div v-if="newTestCases.length > 0">
              <h4 class="text-sm font-semibold mb-2 text-primary">
                New Test Cases ({{ newTestCases.length }})
              </h4>
              <div class="space-y-2">
                <TestCaseEditor v-for="(testCase, index) in newTestCases" :key="index" :test-case="testCase"
                  :index="index" :title="`Test Case ${existingTestCases.length + index + 1}`" :removable="true"
                  @remove="removeNewTestCase(index)" />
              </div>
            </div>

            <!-- No Test Cases Message -->
            <div v-if="existingTestCases.length === 0 && newTestCases.length === 0"
              class="text-center py-8 text-base-content/70">
              <Icon icon="fa7-solid:inbox" width="32" class="mx-auto mb-2" />
              <p>No test cases yet</p>
              <p class="text-sm mt-1">
                Import from ZIP or add manually
              </p>
            </div>

            <!-- ZIP Format Guide -->
            <div class="alert alert-info">
              <Icon icon="fa7-solid:circle-info" width="20" />
              <div class="text-sm">
                <p class="font-semibold">ZIP Format Guide:</p>
                <p>
                  Create folders (e.g., test1, test2) with in.txt and out.txt
                  in each folder
                </p>
              </div>
            </div>
          </div>

          <div class="divider"></div>

          <!-- Action Buttons -->
          <div class="flex justify-end gap-2">
            <button class="btn" @click="handleCancel" :disabled="isSaving">
              <Icon icon="fa7-solid:xmark" width="16" />
              Cancel
            </button>
            <button class="btn btn-primary" @click="handleSave" :disabled="isSaving">
              <span v-if="isSaving" class="loading loading-spinner loading-sm"></span>
              <Icon v-else icon="fa7-solid:floppy-disk" width="16" />
              {{ isEditMode ? "Save Changes" : "Create Problem" }}
            </button>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
