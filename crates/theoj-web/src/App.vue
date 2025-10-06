<script setup lang="ts">
import Header from "./components/Header.vue";
</script>

<template>
  <div class="min-h-screen flex flex-col">
    <Header />
    <div class="flex-1 mx-auto mt-6 w-full max-w-6xl px-8 lg:px-16 flex flex-col">
      <router-view />
    </div>
    <Footer />
  </div>
</template>

<style scoped></style>
