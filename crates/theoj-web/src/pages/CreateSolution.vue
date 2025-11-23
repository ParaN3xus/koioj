<script setup lang="ts">
import { Icon } from "@iconify/vue";
import { onMounted, ref } from "vue";
import { useRoute, useRouter } from "vue-router";
import PreviewableTextEdit from "@/components/PreviewableTextEdit.vue";
import { useApiErrorHandler } from "@/composables/useApiErrorHandler.mjs";
import { buildPath, routeMap } from "@/routes.mjs";
import type { GetProblemResponse } from "@/theoj-api";
import { ProblemService } from "@/theoj-api";

const route = useRoute();
const router = useRouter();
const { handleApiError } = useApiErrorHandler();

const problemId = route.params.id as string;
const problem = ref<GetProblemResponse | null>(null);
const loading = ref(true);
const submitting = ref(false);

const formData = ref({
  title: "",
  content: "",
});

const showContentPreview = ref(false);

const fetchProblem = async () => {
  try {
    loading.value = true;
    problem.value = await ProblemService.getProblem(problemId);
  } catch (e) {
    handleApiError(e)
  } finally {
    loading.value = false;
  }
};

const handleSubmit = async () => {
  if (!formData.value.title.trim() || !formData.value.content.trim()) {
    alert("Please fill in all required fields");
    return;
  }

  try {
    submitting.value = true;
    await ProblemService.createSolution(problemId, {
      title: formData.value.title,
      content: formData.value.content,
    });

    router.push(buildPath(routeMap.solution.path, { id: problemId }));
  } catch (e) {
    handleApiError(e);
  } finally {
    submitting.value = false;
  }
};

const handleCancel = () => {
  router.push(buildPath(routeMap.problem.path, { id: problemId }));
};

onMounted(() => {
  fetchProblem();
});
</script>

<template>
  <div class="container mx-auto max-w-6xl p-4">
    <div class="card bg-base-100 shadow-xl">
      <div class="card-body">
        <!-- Header -->
        <h1 class="text-3xl font-bold">Create Solution</h1>

        <!-- Loading State -->
        <div v-if="loading" class="flex justify-center py-12">
          <span class="loading loading-spinner loading-lg"></span>
        </div>

        <!-- Form -->
        <form v-else @submit.prevent="handleSubmit" class="space-y-6">
          <!-- Title -->
          <div class="form-control">
            <label class="label">
              <span class="label-text font-semibold">
                Title
                <span class="text-error">*</span>
              </span>
            </label>
            <input v-model="formData.title" type="text" placeholder="Enter solution title"
              class="input input-bordered w-full" required />
          </div>

          <!-- Content -->
          <div class="form-control">
            <label class="label">
              <span class="label-text font-semibold">
                Content
                <span class="text-error">*</span>
              </span>
              <label class="label cursor-pointer gap-2">
                <span class="label-text-alt">Preview</span>
                <input type="checkbox" v-model="showContentPreview" class="toggle toggle-sm" />
              </label>
            </label>
            <PreviewableTextEdit v-model="formData.content" placeholder="Enter solution content (Markdown supported)"
              :rows="20" :show-preview="showContentPreview" :required="true" />
            <label class="label">
              <span class="label-text-alt text-base-content/70 inline-flex items-center gap-1">
                <Icon icon="fa6-solid:circle-info" class="w-4 h-4 inline" />
                Markdown is supported. You can use code blocks, images, and other Markdown syntax.
              </span>
            </label>
          </div>

          <!-- Actions -->
          <div class="flex gap-4 justify-end">
            <button type="button" @click="handleCancel" class="btn" :disabled="submitting">
              <Icon icon="fa6-solid:xmark" class="w-5 h-5" />
              Cancel
            </button>
            <button type="submit" class="btn btn-primary" :disabled="submitting">
              <span v-if="submitting" class="loading loading-spinner"></span>
              <Icon v-else icon="fa6-solid:check" class="w-5 h-5" />
              {{ submitting ? "Creating..." : "Create Solution" }}
            </button>
          </div>
        </form>
      </div>
    </div>
  </div>
</template>
