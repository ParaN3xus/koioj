<script setup lang="ts">
import { Icon } from "@iconify/vue";
import { computed, onMounted, ref, watch } from "vue";
import EntityLink from "@/components/EntityLink.vue";
import { useApiErrorHandler } from "@/composables/useApiErrorHandler.mjs";
import { useContestPasswordStore } from "@/stores/contestPassword.mjs";
import type {
  GetContestRankingResponse,
  GetOverallRankingResponse,
} from "@/theoj-api";
import { ContestService } from "@/theoj-api";

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

const isMultiContest = computed(() => contestIdsArray.value.length > 1);

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
        throw new Error("invalid contestId")
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

    <table v-else-if="!isMultiContest && rankingData" class="table">
      <thead>
        <tr>
          <th>Rank</th>
          <th>User</th>
          <th>Solved</th>
          <th>Penalty</th>
          <th v-for="(_, index) in problemIds" :key="index">
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
          <td v-for="result in item.problemResults" :key="result.problemId">
            <div v-if="result.accepted" class="text-success text-center">
              <Icon icon="fa6-solid:check" class="text-xl" />
              <div class="text-xs">{{ result.attempts }}</div>
            </div>
            <div v-else-if="result.attempts > 0" class="text-error text-center">
              <Icon icon="fa6-solid:xmark" class="text-xl" />
              <div class="text-xs">{{ result.attempts }}</div>
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
