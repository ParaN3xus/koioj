import { createApp } from "vue";
import "./style.css";
import { createPinia } from "pinia";
import { createRouter, createWebHistory } from "vue-router";
import Toast from "vue-toastification";
import App from "./App.vue";
import "vue-toastification/dist/index.css";
import { routes } from "./routes.mts";

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
  position: "bottom-right",
  timeout: 2000,
  closeOnClick: false,
  pauseOnFocusLoss: false,
  pauseOnHover: true,
  draggable: true,
  draggablePercent: 0.6,
  showCloseButtonOnHover: false,
  hideProgressBar: false,
  closeButton: "button",
  icon: true,
  rtl: false,
  transition: "custom-toast",
  maxToasts: 20,
  newestOnTop: true,
};

createApp(App).use(router).use(pinia).use(Toast, toastOptions).mount("#app");
