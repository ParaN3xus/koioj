<script setup lang="ts">
import { Icon } from "@iconify/vue";
import { nextTick, type Ref, ref, watch } from "vue";
import EntityLink from "@/components/EntityLink.vue";
import { useApiErrorHandler } from "@/composables/useApiErrorHandler.mjs";
import { useContestPasswordPrompt } from "@/composables/useContestPasswordPrompt.mjs";
import { type ContestInfo, ContestService } from "@/koioj-api";
import { useContestPasswordStore } from "@/stores/contestPassword.mjs";
import { formatDateTime } from "@/utils.mjs";

const modelValue = defineModel<Array<ContestInfo>>({ required: true });

const { handleApiError } = useApiErrorHandler();
const contestPasswordStore = useContestPasswordStore();

const contestIdInput: Ref<number | null> = ref(null);
const isLoadingContest = ref(false);
const previewContest = ref<ContestInfo | null>(null);
const contestIdInputRef = ref<HTMLInputElement | null>(null);

let previewAbortController: AbortController | null = null;

watch(contestIdInput, async (newId) => {
  // Cancel previous request
  if (previewAbortController) {
    previewAbortController.abort();
    previewAbortController = null;
  }

  if (!newId || modelValue.value.some((c) => c.contestId === newId)) {
    previewContest.value = null;
    isLoadingContest.value = false;
    return;
  }

  previewAbortController = new AbortController();
  const currentController = previewAbortController;
  isLoadingContest.value = true;

  try {
    const response = await ContestService.getContest(
      newId,
      contestPasswordStore.getPassword(newId),
    );

    // Check if this request was cancelled
    if (currentController.signal.aborted) {
      return;
    }

    previewContest.value = {
      contestId: Number(response.contestId),
      name: response.name,
      beginTime: response.beginTime,
      endTime: response.endTime,
    };
  } catch (error) {
    // Ignore abort errors
    if (currentController.signal.aborted) {
      return;
    }
    previewContest.value = null;
  } finally {
    if (!currentController.signal.aborted) {
      isLoadingContest.value = false;
    }
  }
});

const handleAddContest = async (event?: Event) => {
  event?.preventDefault();
  if (!contestIdInput.value) {
    return;
  }
  // if already exists
  if (modelValue.value.some((c) => c.contestId === contestIdInput.value)) {
    return;
  }
  // use preview data
  if (
    previewContest.value &&
    previewContest.value.contestId === contestIdInput.value
  ) {
    modelValue.value.push(previewContest.value);
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

    modelValue.value.push({
      contestId: Number(response.contestId),
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
  modelValue.value.splice(index, 1);
};
</script>

<template>
  <div class="form-control">
    <label class="label">
      <span class="label-text font-semibold">
        Contests ({{ modelValue.length }})
      </span>
    </label>
    <!-- Input for adding contests -->
    <div class="join w-full">
      <input ref="contestIdInputRef" v-model.number="contestIdInput" type="number" placeholder="Enter contest ID"
        class="input input-bordered join-item flex-1" @keydown.enter.prevent="handleAddContest" />
      <button type="button" class="btn btn-primary join-item" :disabled="isLoadingContest || !contestIdInput"
        @click="handleAddContest">
        <span v-if="isLoadingContest" class="loading loading-spinner loading-sm"></span>
        <Icon v-else icon="fa6-solid:plus" width="16" />
        Add
      </button>
    </div>
    <!-- Selected contests table -->
    <div v-if="modelValue.length > 0 || previewContest" class="space-y-2">
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
            <tr v-for="(contest, index) in modelValue" :key="contest.contestId">
              <td>
                {{ contest.contestId }}
              </td>
              <td>
                <EntityLink entity-type="contest" :entity-id="contest.contestId">
                  {{ contest.name }}
                </EntityLink>
              </td>
              <td>{{ formatDateTime(contest.beginTime) }}</td>
              <td>{{ formatDateTime(contest.endTime) }}</td>
              <td class="text-right">
                <button type="button" class="btn btn-ghost btn-sm btn-circle" @click="handleRemoveContest(index)">
                  <Icon icon="fa6-solid:xmark" class="text-lg" />
                </button>
              </td>
            </tr>
            <tr v-if="previewContest" class="opacity-50">
              <td>
                {{ previewContest.contestId }}
              </td>
              <td>
                <EntityLink entity-type="contest" :entity-id="previewContest.contestId">
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
</template>
