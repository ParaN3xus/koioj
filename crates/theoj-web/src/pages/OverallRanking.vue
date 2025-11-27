<script setup lang="ts">
import { computed, ref } from "vue";
import ContestRanking from "@/components/ContestRanking.vue";
import ContestSelector from "@/components/ContestSelector.vue";
import type { ContestInfo } from "@/theoj-api";

const selectedContests = ref<Array<ContestInfo>>([]);
const contestIds = computed(() =>
  selectedContests.value.map((c) => c.contestId),
);
</script>

<template>
  <div class="container mx-auto max-w-6xl">
    <div class="card bg-base-100 shadow-xl">
      <div class="card-body">
        <h2 class="card-title">Select Contests</h2>
        <ContestSelector v-model="selectedContests" />
      </div>
    </div>

    <div v-if="contestIds.length > 0" class="card bg-base-100 shadow-xl mt-6">
      <div class="card-body">
        <h2 class="card-title">Overall Ranking</h2>
        <ContestRanking :contest-ids="contestIds" />
      </div>
    </div>
  </div>
</template>
