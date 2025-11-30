<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import EntityLink from "@/components/EntityLink.vue";
import { useApiErrorHandler } from "@/composables/useApiErrorHandler.mjs";
import type {
  GetContestRankingResponse,
  GetOverallRankingResponse,
} from "@/koioj-api";
import { ContestService } from "@/koioj-api";
import { useContestPasswordStore } from "@/stores/contestPassword.mjs";
import { formatDateTime } from "@/utils.mjs";

interface Props {
  contestIds: number | number[];
  problemIds?: number[];
}

const props = defineProps<Props>();
const { handleApiError } = useApiErrorHandler();
const contestPasswordStore = useContestPasswordStore();

const isLoading = ref(true);
const rankingData = ref<
  GetContestRankingResponse | GetOverallRankingResponse | null
>(null);

const contestIdsArray = computed(() =>
  Array.isArray(props.contestIds) ? props.contestIds : [props.contestIds],
);

const isMultiContest = computed(() => Array.isArray(props.contestIds));

const getProblemCellStyle = (attempts: number, accepted: boolean) => {
  if (!accepted) {
    if (attempts !== 0) {
      return {
        backgroundColor: "#ff4444",
        color: "#ffffff",
      };
    }
    return {};
  }

  const saturation = Math.max(30, 80 - attempts * 10);
  return {
    backgroundColor: `hsl(120, ${saturation}%, 45%)`,
    color: "#ffffff",
  };
};

const loadRankingData = async () => {
  try {
    isLoading.value = true;

    if (isMultiContest.value) {
      const response = await ContestService.getOverallRanking(
        contestIdsArray.value,
      );
      rankingData.value = response;
    } else {
      const contestId = contestIdsArray.value[0];
      if (!contestId) {
        throw new Error("invalid contestId");
      }
      const storedPassword = contestPasswordStore.getPassword(contestId);
      const response = await ContestService.getContestRanking(
        contestId,
        storedPassword || null,
      );
      rankingData.value = response;
    }
  } catch (e) {
    handleApiError(e);
  } finally {
    isLoading.value = false;
  }
};

watch(() => props.contestIds, loadRankingData, { deep: true });

onMounted(loadRankingData);
</script>

<template>
  <div class="overflow-x-auto">
    <div v-if="isLoading" class="flex justify-center py-8">
      <span class="loading loading-spinner loading-lg"></span>
    </div>

    <table v-else-if="!isMultiContest && rankingData" class="table table-pin-rows table-pin-cols">
      <thead>
        <tr>
          <th class="min-w-16">Rank</th>
          <th class="min-w-32">User</th>
          <th class="min-w-20">Solved</th>
          <th class="min-w-20">Penalty</th>
          <th v-for="(_, index) in problemIds" :key="index" class="min-w-28">
            {{ String.fromCharCode(65 + index) }}
          </th>
        </tr>
      </thead>
      <tbody>
        <tr v-for="(item, index) in (rankingData as GetContestRankingResponse).rankings" :key="item.userId">
          <td>{{ index + 1 }}</td>
          <td>
            <EntityLink entity-type="user" :entity-id="item.userId">
              {{ item.username }}
            </EntityLink>
          </td>
          <td>{{ item.solvedCount }}</td>
          <td>{{ item.totalPenalty }}</td>
          <td v-for="result in item.problemResults" :key="result.problemId"
            :style="getProblemCellStyle(result.attempts, result.accepted)">
            <div v-if="result.accepted" class="flex flex-col items-center font-bold">
              {{ `+${result.attempts - 1}` }}
              <div v-if="result.acceptedTime" class="text-xs text-center font-normal">
                {{ formatDateTime(result.acceptedTime).replace(", ", "\n") }}
              </div>
            </div>
            <div v-else-if="result.attempts > 0" class="flex flex-col items-center font-bold">
              {{ `-${result.attempts}` }}
            </div>
            <div v-else class="text-center opacity-30">-</div>
          </td>
        </tr>
      </tbody>
    </table>

    <table v-else-if="isMultiContest && rankingData" class="table">
      <thead>
        <tr>
          <th>Rank</th>
          <th>User</th>
          <th>Contests</th>
          <th>Total Solved</th>
          <th>Total Penalty</th>
        </tr>
      </thead>
      <tbody>
        <tr v-for="(item, index) in (rankingData as GetOverallRankingResponse).rankings" :key="item.userId">
          <td>{{ index + 1 }}</td>
          <td>
            <EntityLink entity-type="user" :entity-id="item.userId">
              {{ item.username }}
            </EntityLink>
          </td>
          <td>{{ item.contestCount }}</td>
          <td>{{ item.totalSolved }}</td>
          <td>{{ item.totalPenalty }}</td>
        </tr>
      </tbody>
    </table>
  </div>
</template>
