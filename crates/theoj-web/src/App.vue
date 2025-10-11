<script setup lang="ts">
import { OpenAPI } from "@/theoj-api";
import Footer from "./components/Footer.vue";
import Header from "./components/Header.vue";

if (import.meta.env.DEV) {
  OpenAPI.BASE = "http://localhost:8080";
} else {
  OpenAPI.BASE = window.API_ROOT || "";
}
</script>

<template>
  <div class="min-h-screen flex flex-col bg-base-200">
    <Header />
    <div class="flex-1 mx-auto w-full max-w-6xl px-8 lg:px-16 flex flex-col">
      <router-view />
    </div>
    <Footer />
  </div>
</template>

<style scoped></style>
