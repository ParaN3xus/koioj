<script setup lang="ts">
import { Icon } from "@iconify/vue";
import ThemeToggle from './ThemeToggle.vue';

const navButtons = [
  { to: '/problem', text: 'Problem', icon: 'fa7-solid:book' },
  { to: '/contest', text: 'Contest', icon: 'fa7-solid:trophy' },
  { to: '/training', text: 'Training', icon: 'fa7-solid:chart-line' },
];
const siteTitle = "TheOJ"
</script>

<template>
  <header class="bg-base-100 sticky top-0 z-[1] bg-opacity-80 backdrop-blur shadow-lg">
    <div class="navbar mx-auto max-w-6xl lg:px-16">
      <div class="navbar-start lg:hidden">
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

      <div class="navbar-start hidden lg:flex">
        <RouterLink class="btn btn-ghost text-xl font-bold mr-8" to="/">{{ siteTitle }}</RouterLink>
        <div class="hidden lg:flex lg:space-x-2">
          <RouterLink v-for="item in navButtons" :key="item.to" class="btn btn-ghost w-18 text-base" :to="item.to">
            <Icon :icon="item.icon" width="20" class="mr-1" />
            {{ item.text }}
          </RouterLink>
        </div>
      </div>

      <div class="navbar-center lg:hidden">
        <RouterLink class="btn btn-ghost text-xl font-bold" to="/">{{ siteTitle }}</RouterLink>
      </div>

      <div class="navbar-end space-x-2">
        <ThemeToggle />
      </div>
    </div>
  </header>
</template>
