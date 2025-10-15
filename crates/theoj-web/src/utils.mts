import { useToast } from "vue-toastification";
import type { ApiError } from "./theoj-api";

const toast = useToast();

export function handleApiError(e: unknown) {
  const err = e as ApiError;
  const codeMsg = err.message;

  try {
    const body = typeof err.body === 'string' ? JSON.parse(err.body) : err.body;
    const message = body?.message || 'unknown';

    toast.error(`${codeMsg}: ${message}`);
  } catch {
    toast.error(`${codeMsg}: ${typeof err.body === 'string' ? err.body : 'unknown'}`);
  }
}
