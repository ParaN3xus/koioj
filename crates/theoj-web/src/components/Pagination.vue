<script setup lang="ts">
import { Icon } from '@iconify/vue';
import { computed } from 'vue';

interface Props {
  currentPage: number;
  lastPage: number;
}

const props = defineProps<Props>();

const emit = defineEmits<{
  pageChange: [page: number]
}>();

function getPageNumbers(currentPage: number, lastPage: number) {
  const delta = 2;
  const range = [];
  const rangeWithDots: (number | string)[] = [];

  for (
    let i = Math.max(2, currentPage - delta);
    i <= Math.min(lastPage - 1, currentPage + delta);
    i++
  ) {
    range.push(i);
  }

  if (currentPage - delta > 2) {
    rangeWithDots.push(1, '...');
  } else {
    rangeWithDots.push(1);
  }

  rangeWithDots.push(...range);

  if (currentPage + delta < lastPage - 1) {
    rangeWithDots.push('...', lastPage);
  } else if (lastPage > 1) {
    rangeWithDots.push(lastPage);
  }

  return rangeWithDots;
}

const pageNumbers = computed(() =>
  getPageNumbers(props.currentPage, props.lastPage)
);

const canGoPrev = computed(() => props.currentPage > 1);
const canGoNext = computed(() => props.currentPage < props.lastPage);

const pageBtnClasses = 'join-item btn btn-outline';
const smallPageBtnClasses = 'btn btn-outline btn-sm';

function handlePageChange(page: number) {
  if (page >= 1 && page <= props.lastPage) {
    emit('pageChange', page);
  }
}
</script>

<template>
  <div>
    <!-- Desktop -->
    <div class="hidden sm:block">
      <div class="flex justify-center mt-8">
        <div class="join">
          <button :class="[pageBtnClasses, { 'btn-disabled': !canGoPrev }]" @click="handlePageChange(currentPage - 1)"
            :disabled="!canGoPrev">
            <Icon icon="fa6-solid:chevron-left" width="16" />
            Prev
          </button>

          <template v-for="(pageNum, index) in pageNumbers" :key="index">
            <button v-if="pageNum === '...'" :class="[pageBtnClasses, 'btn-disabled']" disabled>
              ...
            </button>
            <button v-else-if="pageNum === currentPage" :class="[pageBtnClasses, 'btn-active']">
              {{ pageNum }}
            </button>
            <button v-else :class="pageBtnClasses" @click="handlePageChange(pageNum as number)">
              {{ pageNum }}
            </button>
          </template>

          <button :class="[pageBtnClasses, { 'btn-disabled': !canGoNext }]" @click="handlePageChange(currentPage + 1)"
            :disabled="!canGoNext">
            Next
            <Icon icon="fa6-solid:chevron-right" width="16" />
          </button>
        </div>
      </div>
    </div>

    <!-- Mobile -->
    <div class="flex justify-between mt-4 sm:hidden">
      <button :class="[smallPageBtnClasses, { 'btn-disabled': !canGoPrev }]" @click="handlePageChange(currentPage - 1)"
        :disabled="!canGoPrev">
        <Icon icon="fa6-solid:chevron-left" width="12" />
        Prev
      </button>

      <span class="flex items-center text-sm">
        {{ currentPage }} / {{ lastPage }}
      </span>

      <button :class="[smallPageBtnClasses, { 'btn-disabled': !canGoNext }]" @click="handlePageChange(currentPage + 1)"
        :disabled="!canGoNext">
        Next
        <Icon icon="fa6-solid:chevron-right" width="12" />
      </button>
    </div>
  </div>
</template>
