import { createApp } from "vue";
import "./style.css";

import { createRouter, createWebHistory } from "vue-router";
import App from "./App.vue";
import Index from "./pages/Index.vue";
import NotFound from "./pages/NotFound.vue";

const routes = [
  {
    path: "/",
    name: "TheOJ",
    component: Index,
  },
  {
    path: '/:pathMatch(.*)*',
    name: 'NotFound',
    component: NotFound
  }
];

const router = createRouter({
  history: createWebHistory(),
  routes,
});

createApp(App).use(router).mount("#app");
