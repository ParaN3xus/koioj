<script setup lang="ts">
import { ref } from 'vue'
import { useRouter } from 'vue-router'
import { useUserStore } from "@/user.mts";

const router = useRouter()
const userStore = useUserStore()

const email = ref('')
const phone = ref('')
const username = ref('')
const userCode = ref('')
const password = ref('')
const loading = ref(false)

const handleRegister = async () => {
  loading.value = true

  try {
    await userStore.register(
      email.value.trim(),
      phone.value.trim(),
      username.value.trim(),
      userCode.value.trim(),
      password.value)
    router.push('/')
  } catch (_error) {
    console.error(_error)
  } finally {
    loading.value = false
  }
}
</script>

<template>
  <div class="card mx-auto my-auto w-1/2 min-w-96 max-w-6xl bg-base-100 shadow-xl items-center justify-center">
    <div class="card-body w-full ">
      <h2 class="card-title text-2xl font-bold text-center justify-center mb-6">
        Register
      </h2>

      <form @submit.prevent="handleRegister">
        <div class="form-control mb-4">
          <label class="label">
            <span class="label-text">Email</span>
          </label>
          <input v-model="email" type="text" autocomplete="email" placeholder="Enter your email"
            class="input input-bordered w-full" required />
        </div>

        <div class="form-control mb-4">
          <label class="label">
            <span class="label-text">Phone Number</span>
          </label>
          <input v-model="phone" type="text" autocomplete="phone" placeholder="Enter your phone number"
            class="input input-bordered w-full" required />
        </div>

        <div class="form-control mb-4">
          <label class="label">
            <span class="label-text">Username</span>
          </label>
          <input v-model="username" type="text" autocomplete="username" placeholder="Enter your username"
            class="input input-bordered w-full" required />
        </div>

        <div class="form-control mb-4">
          <label class="label">
            <span class="label-text">Student Number or Staff Number</span>
          </label>
          <input v-model="userCode" type="text" placeholder="Enter your student number or staff number"
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
            {{ loading ? 'Registering...' : 'Register' }}
          </button>
        </div>
      </form>

      <div class="divider">Or</div>
      <div class="text-center">
        <a href="/register" class="link">Don't have an account? Register now</a>
      </div>

    </div>
  </div>
</template>
