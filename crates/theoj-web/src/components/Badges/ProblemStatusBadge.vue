<script setup lang="ts">
import { onMounted, ref } from "vue";
import SubmissionResultBadge from "@/components/Badges/SubmissionResultBadge.vue";
import { useApiErrorHandler } from "@/composables/useApiErrorHandler.mjs";
import { type GetAcStatusResponse, ProblemService } from "@/theoj-api";

interface Props {
  problemId: string;
  contestId?: number;
}

const props = defineProps<Props>();
const { handleApiError } = useApiErrorHandler();

const statusData = ref<GetAcStatusResponse | null>(null);
const loading = ref(true);

onMounted(async () => {
  try {
    statusData.value = await ProblemService.getAcStatus(
      props.problemId,
      props.contestId
    );
  } catch (error) {
    handleApiError(error);
  } finally {
    loading.value = false;
  }
});
</script>

<template>
  <div v-if="!loading && statusData?.tried && statusData?.status">
    <SubmissionResultBadge :result="statusData.status" />
  </div>
  <span v-else class="text-sm opacity-50">Not attempted</span>
</template>
