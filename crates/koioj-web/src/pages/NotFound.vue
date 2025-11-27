<script setup lang="ts">
import { RouterLink } from "vue-router";
import { routeMap } from "@/routes.mjs";
</script>

<template>
  <div class="flex-1 flex items-center justify-center min-h-[60vh]">
    <div class="text-center px-4">
      <h1 class="text-4xl md:text-5xl font-bold text-base-content mb-4">
        Page Not Found
      </h1>
      <p class="text-base md:text-lg text-base-content/70 mb-8 max-w-md mx-auto">
        Sorry, the page you are looking for doesn't exist or has been moved.
      </p>
      <RouterLink :to="routeMap.index.path" class="btn btn-outline btn-neutral btn-lg">
        Go to Homepage
      </RouterLink>
    </div>
  </div>
</template>
