import { createApp } from "vue";
import "./style.css";

import { createPinia } from "pinia";
import {
  createRouter,
  createWebHistory,
  type RouteRecordRaw,
} from "vue-router";
import Toast from 'vue-toastification'

import App from "./App.vue";
import Index from "./pages/Index.vue";
import Login from "./pages/Login.vue";
import NotFound from "./pages/NotFound.vue";
import Profile from "./pages/Profile.vue";
import Register from "./pages/Register.vue";

import 'vue-toastification/dist/index.css'


const routes: RouteRecordRaw[] = [
  {
    path: "/",
    name: "index",
    component: Index,
    meta: {
      title: "TheOJ - The Online Judge Platform",
    },
  },
  {
    path: "/users/login",
    name: "login",
    component: Login,
    meta: {
      title: "Login - TheOJ",
    },
  },
  {
    path: "/users/register",
    name: "register",
    component: Register,
    meta: {
      title: "Register - TheOJ",
    },
  },
  {
    path: "/users/profile/:id",
    name: "profile",
    component: Profile,
    meta: {
      title: "Profile - TheOJ",
    },
  },
  {
    path: "/:pathMatch(.*)*",
    name: "not-found",
    component: NotFound,
    meta: {
      title: "404 Not Found - TheOJ",
    },
  },
];

const router = createRouter({
  history: createWebHistory(),
  routes,
});

router.beforeEach((to, _from, next) => {
  document.title = (to.meta.title as string) || "TheOJ";
  next();
});

const pinia = createPinia();

const toastOptions = {
  position: 'bottom-right',
  timeout: 2000,
  closeOnClick: false,
  pauseOnFocusLoss: false,
  pauseOnHover: true,
  draggable: true,
  draggablePercent: 0.6,
  showCloseButtonOnHover: false,
  hideProgressBar: false,
  closeButton: 'button',
  icon: true,
  rtl: false,
  transition: 'custom-toast',
  maxToasts: 20,
  newestOnTop: true,
}

createApp(App).use(router).use(pinia).use(Toast, toastOptions).mount("#app");
