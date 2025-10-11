import { createApp } from "vue";
import "./style.css";

import { createPinia } from "pinia";
import { createRouter, createWebHistory, type RouteRecordRaw } from "vue-router";
import App from "./App.vue";
import Index from "./pages/Index.vue";
import Login from "./pages/Login.vue";
import NotFound from "./pages/NotFound.vue";
import Register from "./pages/Register.vue";

const routes: RouteRecordRaw[] = [
  {
    path: "/",
    name: "index",
    component: Index,
    meta: {
      title: "TheOJ - The Online Judge Platform"
    }
  },
  {
    path: "/user/login",
    name: "login",
    component: Login,
    meta: {
      title: "Login - TheOJ"
    }
  },
  {
    path: "/user/register",
    name: "register",
    component: Register,
    meta: {
      title: "Register - TheOJ"
    }
  },
  {
    path: "/:pathMatch(.*)*",
    name: "not-found",
    component: NotFound,
    meta: {
      title: "404 Not Found - TheOJ"
    }
  },
];

const router = createRouter({
  history: createWebHistory(),
  routes,
});

router.beforeEach((to, _from, next) => {
  document.title = (to.meta.title as string) || 'TheOJ';
  next();
});

const pinia = createPinia();

createApp(App).use(router).use(pinia).mount("#app");
