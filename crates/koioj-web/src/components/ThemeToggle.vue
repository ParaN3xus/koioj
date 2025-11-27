<script setup lang="ts">
import { Icon } from "@iconify/vue";
import { themeChange } from "theme-change";
import { onMounted, ref } from "vue";

const isLightTheme = ref(true);

onMounted(() => {
  themeChange();

  const savedTheme: string = localStorage.getItem("theme") || "light";
  document.documentElement.setAttribute("data-theme", savedTheme);

  isLightTheme.value = savedTheme === "light";
});

const handleThemeChange = (): void => {
  const newTheme: string = isLightTheme.value ? "light" : "dark";
  localStorage.setItem("theme", newTheme);
};
</script>

<template>
  <label class="swap swap-rotate btn btn-ghost btn-circle">
    <input type="checkbox" data-toggle-theme="dark,light" data-act-class="ACTIVECLASS" id="theme-toggle"
      v-model="isLightTheme" @change="handleThemeChange" />
    <Icon icon="fa7-solid:sun" width="20" class="swap-on text-yellow-500" />
    <Icon icon="fa7-solid:moon" width="20" class="swap-off text-gray-500" />
  </label>
</template>