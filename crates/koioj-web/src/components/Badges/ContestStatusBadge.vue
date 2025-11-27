<script setup lang="ts">
import { computed } from "vue";

interface Props {
  beginTime: string;
  endTime: string;
}

const props = defineProps<Props>();

type ContestStatus = "upcoming" | "ongoing" | "ended";

const status = computed<ContestStatus>(() => {
  const now = new Date();
  const begin = new Date(props.beginTime);
  const end = new Date(props.endTime);

  if (now < begin) {
    return "upcoming";
  } else if (now >= begin && now <= end) {
    return "ongoing";
  } else {
    return "ended";
  }
});
</script>

<template>
  <span v-if="status === 'ongoing'" class="badge badge-warning">
    Ongoing
  </span>
  <span v-else-if="status === 'upcoming'" class="text-sm opacity-50">
    Not Started
  </span>
  <span v-else class="text-sm opacity-50">
    Ended
  </span>
</template>
