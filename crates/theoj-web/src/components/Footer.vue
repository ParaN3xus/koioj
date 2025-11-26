<script setup lang="ts">
import { Icon } from "@iconify/vue";

const currentYear = new Date().getFullYear();
const siteTitle = "TheOJ";
</script>

<template>
  <footer class="bg-base-100 border-t border-base-300 mt-auto">
    <div class="mx-auto max-w-6xl px-4 py-4 md:px-8 md:py-6">
      <div class="flex flex-col md:flex-row justify-between items-center gap-3 md:gap-4">
        <div class="flex flex-col md:flex-row items-center gap-1 md:gap-4 text-center md:text-left">
          <span class="text-sm md:text-base font-bold">{{ siteTitle }}</span>
          <span class="text-xs md:text-sm text-base-content/60">
            © {{ currentYear }} {{ siteTitle }}. All rights reserved.
          </span>
        </div>
      </div>
    </div>
  </footer>
</template>
