import { useRouter } from "vue-router";
import InputModal from "@/components/Modal/modals/InputModal.vue";
import { useModal } from "@/components/Modal/useModal.mts";
import { routeMap } from "@/routes.mts";
import { useContestPasswordStore } from "@/stores/contestPassword.mts";

export interface ContestPasswordPromptOptions {
  contestId: number;
  onPasswordSubmit: (password: string) => Promise<void>;
  onCancel?: () => void;
}

export function useContestPasswordPrompt(
  options: ContestPasswordPromptOptions,
) {
  const router = useRouter();
  const contestPasswordStore = useContestPasswordStore();
  const { contestId, onPasswordSubmit, onCancel } = options;

  const promptForPassword = (isWrongPassword = false) => {
    const { open, close } = useModal({
      component: InputModal,
      attrs: {
        title: "Contest Password Required",
        placeholder: "Enter contest password",
        inputType: "password",
        confirmText: "Submit",
        cancelText: "Cancel",
        errorMessage: isWrongPassword
          ? "Incorrect password. Please try again."
          : "",
        initialValue: contestPasswordStore.getPassword(contestId) || "",
        async onConfirm(password: string) {
          close();
          await onPasswordSubmit(password);
        },
        onCancel() {
          if (onCancel) {
            onCancel();
          } else {
            router.push(routeMap.contestList.path);
          }
          close();
        },
      },
    });

    open();
  };

  return {
    promptForPassword,
  };
}
