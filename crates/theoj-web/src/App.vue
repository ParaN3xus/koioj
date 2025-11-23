<script setup lang="ts">
import { OpenAPI } from "@/theoj-api";
import Footer from "./components/Footer.vue";
import Header from "./components/Header.vue";
import ModalContainer from "./components/Modal/ModalContainer.vue";
import { useUserStore } from "./stores/user.mts";

if (import.meta.env.DEV) {
  OpenAPI.BASE = "http://localhost:8080";
} else {
  OpenAPI.BASE = window.API_ROOT || "";
}

const userStore = useUserStore();

if (userStore.isLoggedIn) {
  OpenAPI.TOKEN = userStore.token;
}
</script>

<template>
  <div class="min-h-screen flex flex-col bg-base-200">
    <Header />
    <div class="flex-1 mx-auto w-full max-w-6xl px-8 py-8 flex flex-col">
      <router-view />
    </div>
    <Footer />
  </div>
  <ModalContainer />
</template>

<style scoped></style>
