<script setup lang="ts">
import { onMounted, ref } from "vue";
import { useRouter } from "vue-router";
import { useToast } from "vue-toastification";
import { useApiErrorHandler } from "@/composables/useApiErrorHandler.mjs";
import { routeMap } from "@/routes.mjs";
import { useUserStore } from "@/stores/user.mjs";

const toast = useToast();
const router = useRouter();
const userStore = useUserStore();
const { handleApiError } = useApiErrorHandler();

const identifier = ref("");
const password = ref("");
const loading = ref(false);

const handleLogin = async () => {
  loading.value = true;

  try {
    await userStore.login(identifier.value.trim(), password.value);
    toast.success("Logged in!");
    router.push(routeMap.index.path);
  } catch (e) {
    handleApiError(e);
  } finally {
    loading.value = false;
  }
};

onMounted(() => {
  if (userStore.isLoggedIn) {
    toast.info("Already logged in!");
    router.push(routeMap.index.path);
  }
});
</script>

<template>
  <div class="card mx-auto my-auto w-1/2 min-w-96 max-w-6xl bg-base-100 shadow-xl items-center justify-center">
    <div class="card-body w-full">
      <h2 class="card-title text-2xl font-bold text-center justify-center mb-6">
        Login
      </h2>

      <form @submit.prevent="handleLogin">
        <div class="form-control mb-4">
          <label class="label">
            <span class="label-text">Email or Phone number</span>
          </label>
          <input v-model="identifier" type="text" autocomplete="email" placeholder="Enter your username or email"
            class="input input-bordered w-full" required />
        </div>

        <div class="form-control mb-6">
          <label class="label">
            <span class="label-text">Password</span>
          </label>
          <input v-model="password" type="password" autocomplete="current-password" placeholder="Enter your password"
            class="input input-bordered w-full" required />
        </div>

        <div class="form-control mt-6">
          <button type="submit" class="btn btn-primary w-full" :disabled="loading">
            <div v-if="loading" class="loading loading-spinner" />
            {{ loading ? 'Logging in...' : 'Login' }}
          </button>
        </div>
      </form>

      <div class="divider">Or</div>
      <div class="text-center">
        <a :href="routeMap.register.path" class="link">Don't have an account? Register now</a>
      </div>

    </div>
  </div>
</template>
