<script setup lang="ts">
import { Icon } from "@iconify/vue";
import { computed, onMounted, ref, watch } from "vue";
import { useRoute, useRouter } from "vue-router";
import { useToast } from "vue-toastification";
import ConfirmModal from "@/components/Modal/modals/ConfirmModal.vue";
import { useModal } from "@/components/Modal/useModal.mjs";
import { useApiErrorHandler } from "@/composables/useApiErrorHandler.mjs";
import { routeMap, routes } from "@/routes.mjs";
import { useUserStore } from "@/stores/user.mjs";
import {
  type GetProfileResponse,
  type PutProfileRequest,
  type PutRoleRequest,
  UserRole,
  UserService,
} from "@/theoj-api";
import { parseIntOrNull } from "@/utils.mjs";

const { handleApiError } = useApiErrorHandler();
const router = useRouter();
const toast = useToast();
const userStore = useUserStore();
const route = useRoute();

const profileUserId = computed(() => parseIntOrNull(route.params.id) ?? -1);
const isOwnProfile = computed(() => profileUserId.value === userStore.userId);

const userRole = ref<UserRole | null>(null);
const currentUserRole = ref<UserRole | null>(null);
const profileData = ref<GetProfileResponse | null>(null);
const isLoading = ref(true);

const menuItems = [
  {
    key: "basic",
    label: "Basic Info",
    icon: "fa7-solid:user",
  },
  {
    key: "security",
    label: "Security",
    icon: "fa6-solid:lock",
  },
  {
    key: "delete",
    label: "Delete Account",
    icon: "fa7-solid:remove",
  },
] as const;
type MenuKey = (typeof menuItems)[number]["key"];
const activeTab = ref<MenuKey>("basic");

const canManageRole = computed(() => {
  return currentUserRole.value === UserRole.ADMIN && !isOwnProfile.value;
});

const canViewDetails = computed(() => {
  return (
    currentUserRole.value === UserRole.ADMIN ||
    currentUserRole.value === UserRole.TEACHER
  );
});

const loadUserData = async () => {
  isLoading.value = true;
  try {
    const currentRoleResponse = await UserService.getRole(userStore.userId ?? -1);
    currentUserRole.value = currentRoleResponse.role;
    const roleResponse = await UserService.getRole(profileUserId.value);
    userRole.value = roleResponse.role;
    const profileResponse = await UserService.getProfile(profileUserId.value);
    profileData.value = profileResponse;

    toast.success("Profile loaded!");
  } catch (e) {
    handleApiError(e);
  } finally {
    isLoading.value = false;
  }
};
watch(
  () => route.params.id,
  () => {
    loadUserData();
  },
);

onMounted(() => {
  loadUserData();
});

const userBriefInfo = computed(() => [
  {
    icon: "fa7-solid:envelope",
    value: profileData.value?.email,
  },
  {
    icon: "fa7-solid:phone",
    value: profileData.value?.phone,
  },
]);

type FormField<T> = {
  key: keyof T;
  label: string;
  type: string;
  placeholder: string;
  disabled: boolean;
};

const formFields = computed<FormField<GetProfileResponse>[]>(() => [
  {
    key: "username",
    label: "Username",
    type: "text",
    placeholder: "Username",
    disabled: false,
  },
  {
    key: "email",
    label: "Email",
    type: "email",
    placeholder: "Email",
    disabled: false,
  },
  {
    key: "phone",
    label: "Phone",
    type: "phone",
    placeholder: "Phone",
    disabled: true,
  },
  {
    key: "userCode",
    label: userRole.value === UserRole.TEACHER ? "Staff ID" : "Student ID",
    type: "text",
    placeholder: "ID",
    disabled: true,
  },
]);

const isRoleToggling = ref(false);
const handleRoleToggle = async () => {
  if (!userRole.value || isRoleToggling.value) return;

  isRoleToggling.value = true;
  try {
    const newRole: UserRole =
      userRole.value === UserRole.STUDENT ? UserRole.TEACHER : UserRole.STUDENT;

    const requestBody: PutRoleRequest = {
      userRole: newRole,
    };

    await UserService.putRole(profileUserId.value, requestBody);
    userRole.value = newRole;

    toast.success("Role changed!");
  } catch (e) {
    handleApiError(e);
  } finally {
    isRoleToggling.value = false;
  }
};

const roleText = computed(() => {
  if (!userRole.value) {
    return "unknown";
  }
  return userRole.value.charAt(0).toUpperCase() + userRole.value.slice(1);
});

const roleBadgeClass = computed(() => {
  switch (userRole.value) {
    case UserRole.ADMIN:
      return "badge-primary";
    case UserRole.TEACHER:
      return "badge-secondary";
    case UserRole.STUDENT:
      return "badge-neutral";
    default:
      return "badge-ghost";
  }
});

const handleUpdateBasicInfo = async () => {
  const formData: Record<string, string | undefined | null> = {};

  formFields.value.forEach((field) => {
    if (!field.disabled && profileData.value?.[field.key] !== undefined) {
      formData[field.key] = profileData.value[field.key];
    }
  });

  try {
    await UserService.putProfile(
      userStore.userId ?? -1,
      formData as PutProfileRequest,
    );
    toast.success("Profile updated!");
  } catch (e) {
    handleApiError(e);
  }
};

const handleLogout = async () => {
  userStore.logout();
  toast.success("Logged out!");
  router.push(routeMap.index.path);
};

const curPassword = ref("");
const newPassword = ref("");
const confirmNewpassword = ref("");

const handleUpdatePassword = async () => {
  if (confirmNewpassword.value !== newPassword.value) {
    toast.error("Passwords do not match");
    return;
  }

  try {
    await UserService.changePassword({
      oldPassword: curPassword.value,
      newPassword: newPassword.value,
    });
    toast.success("Password changed!");
  } catch (e) {
    handleApiError(e);
  }
};

const { open: handleDeleteAccount, close: closeDeleteAccountModal } = useModal({
  component: ConfirmModal,
  attrs: {
    title: "Are you sure to delete your account?",
    reverseColors: true,
    reverseOrder: true,
    async onYes() {
      await handleConfirmDeleteAccount();
    },
    onNo() { },
  },
  slots: {
    default:
      "<p>Are you sure you want to delete your account? You will no longer be able to log in to your account.</p>",
  },
});

const handleConfirmDeleteAccount = async () => {
  try {
    await UserService.deleteUser(userStore.userId ?? -1);
    userStore.logout();
    toast.success("User deleted!");
    router.push(routeMap.index.path);
  } catch (e) {
    handleApiError(e);
  }
};
</script>

<template>
  <div class="container mx-auto max-w-6xl">
    <!-- basic info -->
    <div class="card bg-base-100 shadow-xl">
      <div class="card-body">
        <div v-if="isLoading" class="flex items-center justify-center py-8">
          <span class="loading loading-spinner loading-lg"></span>
        </div>

        <div v-else class="flex items-center justify-between">
          <div class="flex items-center gap-8">
            <div class="w-20 h-20 flex items-center justify-center rounded-full bg-base-300">
              <Icon icon="fa7-solid:user" width="32" />
            </div>

            <div>
              <h2 class="text-2xl font-bold">
                {{ profileData?.username }}
                <span class="badge align-middle" :class="roleBadgeClass">
                  {{ roleText }}
                </span>
              </h2>

              <!-- details, only for teachers and admins -->
              <div v-if="canViewDetails" class="mt-2 space-y-1 text-sm text-base-content/70">
                <div v-for="item in userBriefInfo" :key="item.icon" class="flex items-center gap-2">
                  <Icon :icon="item.icon" width="14" />
                  <span>{{ item.value }}</span>
                </div>

                <div class="flex items-center gap-2">
                  <Icon icon="fa7-solid:id-card" width="14" />
                  <span>
                    {{ userRole === UserRole.TEACHER ? 'Staff ID' : 'Student ID' }}:
                    {{ profileData?.userCode }}
                  </span>
                </div>
              </div>
            </div>
          </div>

          <!-- promote or demote, only for admins -->
          <div v-if="canManageRole" class="flex flex-col items-end gap-2 mt-auto">
            <div class="form-control">
              <label class="label cursor-pointer gap-2">
                <span class="label-text">
                  {{ userRole === UserRole.STUDENT ? 'Promote to Teacher' : 'Demote to Student' }}
                </span>
                <input type="checkbox" class="toggle toggle-primary" :checked="userRole === UserRole.TEACHER"
                  @change="handleRoleToggle" />
              </label>
            </div>
          </div>

          <!-- logout -->
          <div v-if="isOwnProfile" class="flex flex-col items-end mt-auto gap-2">
            <button class="btn" @click="handleLogout">
              <Icon icon="fa7-solid:sign-out" width="16" />
              Logout
            </button>
          </div>
        </div>
      </div>
    </div>

    <!-- settings -->
    <div v-if="isOwnProfile && !isLoading" class="card bg-base-100 shadow-xl mt-6">
      <div class="card-body">
        <h3 class="card-title">Settings</h3>

        <div class="flex gap-6 mt-4">
          <!-- tabs -->
          <div class="w-48 flex-shrink-0">
            <ul class="menu bg-base-200 rounded-box space-y-2">
              <li v-for="item in menuItems" :key="item.key">
                <a :class="{ 'active': activeTab === item.key }" @click="activeTab = item.key">
                  <Icon :icon="item.icon" width="16" />
                  {{ item.label }}
                </a>
              </li>
            </ul>
          </div>

          <div class="flex-1">
            <!-- basic info -->
            <div v-if="activeTab === 'basic'" class="space-y-2">
              <h4 class="text-lg font-semibold">Basic Information</h4>

              <div v-for="field in formFields" :key="field.key" class="form-control">
                <label class="label">
                  <span class="label-text">{{ field.label }}</span>
                </label>
                <input :type="field.type" :disabled="field.disabled" :placeholder="field.placeholder"
                  class="input input-bordered" v-model="profileData![field.key]" />
              </div>

              <div>
                <button class="btn btn-primary mt-4" @click="handleUpdateBasicInfo">
                  <Icon icon="fa7-solid:floppy-disk" width="16" />
                  Save Changes
                </button>
              </div>
            </div>

            <!-- security -->
            <div v-if="activeTab === 'security'" class="space-y-2">
              <h4 class="text-lg font-semibold">Change Password</h4>

              <div class="form-control">
                <label class="label">
                  <span class="label-text">Current Password</span>
                </label>
                <input type="password" v-model="curPassword" placeholder="Current password"
                  class="input input-bordered" />
              </div>

              <div class="form-control">
                <label class="label">
                  <span class="label-text">New Password</span>
                </label>
                <input type="password" v-model="newPassword" placeholder="New password" class="input input-bordered" />
              </div>

              <div class="form-control">
                <label class="label">
                  <span class="label-text">Confirm New Password</span>
                </label>
                <input type="password" v-model="confirmNewpassword" placeholder="Confirm new password"
                  class="input input-bordered" />
              </div>

              <div>
                <button class="btn btn-primary mt-4" @click="handleUpdatePassword">
                  <Icon icon="fa7-solid:key" width="16" />
                  Update Password
                </button>
              </div>
            </div>

            <div v-if="activeTab === 'delete'" class="space-y-2">
              <h4 class="text-lg font-semibold">Delete Account</h4>
              <div>
                <button class="btn btn-error mt-4" @click="handleDeleteAccount">
                  <Icon icon="fa7-solid:remove" width="16" />
                  Delete Account
                </button>
              </div>
            </div>

          </div>
        </div>
      </div>
    </div>
  </div>
</template>
