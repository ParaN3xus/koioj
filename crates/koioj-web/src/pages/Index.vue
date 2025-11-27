<script setup lang="ts">
import { Icon } from "@iconify/vue";
import { RouterLink } from "vue-router";
import { routeMap } from "@/routes.mjs";
import { APP_NAME, SOURCE_REPO } from "@/utils.mjs";

interface FeatureCard {
  title: string;
  description: string;
  icon: string;
  route: string;
  color: string;
}

const features: FeatureCard[] = [
  {
    title: "Problems",
    description: "Browse and solve programming problems",
    icon: "fa6-solid:code",
    route: routeMap.problemList.path,
    color: "text-primary",
  },
  {
    title: "Contests",
    description: "Participate in competitive programming contests",
    icon: "fa6-solid:trophy",
    route: routeMap.contestList.path,
    color: "text-secondary",
  },
  {
    title: "Training Plans",
    description: "Follow structured learning paths",
    icon: "fa6-solid:graduation-cap",
    route: routeMap.trainingPlanList.path,
    color: "text-accent",
  },
  {
    title: "Source Code",
    description: "View our open source code on GitHub",
    icon: "fa6-brands:github",
    route: SOURCE_REPO,
    color: "text-info",
  },
];
</script>

<template>
  <div class="container mx-auto max-w-6xl">
    <!-- Hero Section -->
    <div class="card bg-base-100 shadow-xl mb-6">
      <div class="card-body text-center py-12">
        <h1 class="text-5xl font-bold mb-4">
          <Icon icon="fa6-solid:gavel" class="inline-block mr-3" />
          Welcome to {{ APP_NAME }}
        </h1>
        <p class="text-xl text-base-content/70 mb-6">
          The Online Judge Platform for Competitive Programming
        </p>
        <div class="flex gap-4 justify-center">
          <RouterLink :to="routeMap.register.path" class="btn btn-primary btn-lg">
            <Icon icon="fa6-solid:user-plus" class="mr-2" />
            Get Started
          </RouterLink>
          <RouterLink :to="routeMap.problemList.path" class="btn btn-outline btn-lg">
            <Icon icon="fa6-solid:code" class="mr-2" />
            Browse Problems
          </RouterLink>
        </div>
      </div>
    </div>

    <!-- Features Section -->
    <div class="card bg-base-100 shadow-xl">
      <div class="card-body">
        <h2 class="card-title text-3xl mb-6">
          <Icon icon="fa6-solid:compass" class="mr-2" />
          Explore Features
        </h2>
        <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
          <component :is="feature.route.startsWith('http') ? 'a' : RouterLink" v-for="feature in features"
            :key="feature.title" :to="feature.route.startsWith('http') ? undefined : feature.route"
            :href="feature.route.startsWith('http') ? feature.route : undefined"
            :target="feature.route.startsWith('http') ? '_blank' : undefined"
            :rel="feature.route.startsWith('http') ? 'noopener noreferrer' : undefined"
            class="card bg-base-200 hover:bg-base-300 transition-colors cursor-pointer">
            <div class="card-body">
              <h3 class="card-title">
                <Icon :icon="feature.icon" :class="feature.color" class="text-2xl" />
                {{ feature.title }}
              </h3>
              <p class="text-base-content/70">{{ feature.description }}</p>
              <div class="card-actions justify-end">
                <Icon
                  :icon="feature.route.startsWith('http') ? 'fa6-solid:arrow-up-right-from-square' : 'fa6-solid:arrow-right'"
                  class="text-xl" />
              </div>
            </div>
          </component>
        </div>
      </div>
    </div>
  </div>
</template>
