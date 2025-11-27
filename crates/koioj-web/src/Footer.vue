<script setup lang="ts">
import { APP_NAME, SOURCE_REPO } from "@/utils.mjs";

const currentYear = new Date().getFullYear();
</script>

<template>
  <footer class="bg-base-100 border-t border-base-300 mt-auto">
    <div class="mx-auto max-w-6xl px-4 py-4 md:px-8 md:py-6">
      <div class="flex flex-col md:flex-row justify-between items-center gap-3 md:gap-4">
        <div class="flex flex-col md:flex-row items-center gap-1 md:gap-4 text-center md:text-left">
          <span class="text-sm md:text-base font-bold">{{ APP_NAME }}</span>
          <span class="text-xs md:text-sm text-base-content/60">
            © {{ currentYear }} {{ APP_NAME }}. Licensed under <a class="link"
              :href="`${SOURCE_REPO}/blob/main/LICENSE`">GPL-3.0-only</a>
          </span>
        </div>
      </div>
    </div>
  </footer>
</template>
