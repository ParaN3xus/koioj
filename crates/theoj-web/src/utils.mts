import { useRouter } from "vue-router";
import { useToast } from "vue-toastification";
import { routeMap } from "./routes.mts";
import type { ApiError } from "./theoj-api";

export function useApiErrorHandler() {
  const router = useRouter();
  const toast = useToast();

  const handleApiError = (e: unknown) => {
    const err = e as ApiError;
    const codeMsg = err.message;

    try {
      const body = typeof err.body === "string" ? JSON.parse(err.body) : err.body;
      const message = body?.message || err.message;

      toast.error(`${codeMsg}: ${message}`);

      if (err.status === 401 && (message === "missing auth header" || message === "missing auth token")) {
        router.push(routeMap.login.path);
      }
    } catch {
      toast.error(
        `${codeMsg}: ${typeof err.body === "string" ? err.body : "unknown"}`,
      );
    }
  }
  return { handleApiError };
}