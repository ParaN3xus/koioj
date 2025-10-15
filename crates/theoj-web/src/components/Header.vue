<script setup lang="ts">
import { Icon } from "@iconify/vue";
import { useUserStore } from "@/user.mts";
import ThemeToggle from "./ThemeToggle.vue";

const navButtons = [
  { to: "/problem", text: "Problem", icon: "fa7-solid:book" },
  { to: "/contest", text: "Contest", icon: "fa7-solid:trophy" },
  { to: "/training", text: "Training", icon: "fa7-solid:chart-line" },
];
const siteTitle = "TheOJ";

const userStore = useUserStore();

</script>

<template>
  <header class="bg-base-100 sticky top-0 z-[1] bg-opacity-80 backdrop-blur shadow-lg">
    <div class="navbar mx-auto max-w-6xl md:px-8">
      <div class="navbar-start md:hidden">
        <div class="dropdown">
          <div tabindex="0" class="btn btn-ghost btn-circle">
            <Icon icon="fa7-solid:bars" width="20" />
          </div>
          <ul tabindex="0" class="menu dropdown-content bg-base-100 rounded-box z-[1] mt-3 space-y-2 shadow-lg">
            <li v-for="item in navButtons" :key="item.to">
              <RouterLink class="btn btn-ghost w-20" :to="item.to">
                {{ item.text }}
              </RouterLink>
            </li>
          </ul>
        </div>
      </div>

      <div class="navbar-start hidden md:flex md:flex-1">
        <RouterLink class="btn btn-ghost text-xl font-bold mr-8" to="/">{{ siteTitle }}</RouterLink>
        <div class="hidden md:flex md:space-x-2">
          <RouterLink v-for="item in navButtons" :key="item.to" class="btn btn-ghost w-18 text-base" :to="item.to">
            <Icon :icon="item.icon" width="20" class="mr-1" />
            {{ item.text }}
          </RouterLink>
        </div>
      </div>

      <div class="navbar-center md:hidden">
        <RouterLink class="btn btn-ghost text-xl font-bold" to="/">{{ siteTitle }}</RouterLink>
      </div>

      <div class="navbar-end space-x-2 md:flex-none md:w-auto">
        <template v-if="userStore.isLoggedIn">
          <RouterLink class="flex items-center btn btn-ghost btn-circle justify-center rounded-full"
            :to="`/users/profile/${userStore.userId}`">
            <div class="w-10 h-10 flex items-center justify-center rounded-full bg-base-300 ">
              <Icon icon="fa6-solid:user" width="16" />
            </div>
          </RouterLink>
        </template>
        <template v-else>
          <RouterLink class="btn btn-ghost btn-sm" to="/users/login">Login</RouterLink>
          <RouterLink class="btn btn-primary btn-sm" to="/users/register">Register</RouterLink>
        </template>
        <ThemeToggle />
      </div>
    </div>
  </header>
</template>
