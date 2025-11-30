import { createPinia } from "pinia";
import { createApp } from "vue";
import { createRouter, createWebHistory } from "vue-router";
import Toast from "vue-toastification";
import App from "./App.vue";
import { routes } from "./routes.mts";

import "./style.css";
import "katex/dist/katex.min.css";
import "vue-toastification/dist/index.css";
import { APP_NAME } from "./utils.mts";

const router = createRouter({
  history: createWebHistory(),
  routes,
});

router.beforeEach((to, _from, next) => {
  document.title = (to.meta.title as string) || APP_NAME;
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


const app = createApp(App)
document.title = APP_NAME

app
  .use(router)
  .use(pinia)
  .use(Toast, toastOptions)
  .mount("#app");
