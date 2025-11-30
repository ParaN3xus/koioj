<script setup lang="ts">
import { Icon } from "@iconify/vue";
import { computed, onMounted, ref } from "vue";
import { useRoute, useRouter } from "vue-router";
import { useToast } from "vue-toastification";
import { useApiErrorHandler } from "@/composables/useApiErrorHandler.mjs";
import {
  ContestService,
  type GetContestResponse,
  JudgeService,
  Language,
  ProblemService,
  type SubmitRequest,
} from "@/koioj-api";
import { buildPath, routeMap } from "@/routes.mjs";
import { useContestPasswordStore } from "@/stores/contestPassword.mjs";
import { APP_NAME, parseIntOrNull } from "@/utils.mjs";

const STORAGE_KEY_LAST_LANGUAGE = "koioj_last_selected_language";

const route = useRoute();
const router = useRouter();
const toast = useToast();
const { handleApiError } = useApiErrorHandler();
const contestPasswordStore = useContestPasswordStore();

const problemId = computed(() => {
  // Contest mode: /contest/:contestId/problem/:problemId/submit
  if (route.params.problemId) {
    return parseIntOrNull(route.params.problemId) ?? -1;
  }
  // Normal mode: /problem/:id/submit
  return parseIntOrNull(route.params.id) ?? -1;
});

const contestId = computed(() => parseIntOrNull(route.params.contestId));
const isContestMode = computed(() => !!contestId.value);

const code = ref<string>("");
const lang = ref<Language>();
const isSubmitting = ref<boolean>(false);
const problemName = ref<string>("");
const contestData = ref<GetContestResponse | null>(null);
const supportedLanguages = ref<Language[]>([]);

// Language display name mapping
const languageLabels: Record<Language, string> = {
  [Language.C]: "C",
  [Language.CPP]: "C++",
  [Language.JAVA]: "Java",
  [Language.PYTHON]: "Python",
  [Language.GO]: "Go",
  [Language.RUST]: "Rust",
  [Language.JAVASCRIPT]: "JavaScript",
  [Language.TYPESCRIPT]: "TypeScript",
  [Language.CSHARP]: "C#",
  [Language.PHP]: "PHP",
  [Language.RUBY]: "Ruby",
  [Language.SWIFT]: "Swift",
  [Language.KOTLIN]: "Kotlin",
  [Language.SCALA]: "Scala",
  [Language.HASKELL]: "Haskell",
  [Language.LUA]: "Lua",
  [Language.PERL]: "Perl",
  [Language.R]: "R",
  [Language.DART]: "Dart",
  [Language.OBJECTIVEC]: "Objective-C",
};

const languages = computed(() =>
  supportedLanguages.value.map((langValue) => ({
    value: langValue,
    label: languageLabels[langValue] || langValue,
  })),
);

const loadSupportedLanguages = async () => {
  try {
    const response = await JudgeService.getSupportedLanguages();
    supportedLanguages.value = response.languages;

    const lastLang = localStorage.getItem(STORAGE_KEY_LAST_LANGUAGE) as Language | null;
    if (
      lastLang &&
      supportedLanguages.value.includes(lastLang)
    ) {
      lang.value = lastLang;
    } else if (supportedLanguages.value.length > 0) {
      if (!supportedLanguages.value[0]) {
        throw Error("There's no supported languages!");
      }
      lang.value = supportedLanguages.value[0];
    }
  } catch (e) {
    handleApiError(e);
  }
};

const loadProblemAndContestData = async (password?: string) => {
  try {
    const problemResponse = await ProblemService.getProblem(problemId.value);
    problemName.value = problemResponse.name;

    // If in contest mode, also fetch contest data
    if (isContestMode.value && contestId.value) {
      const storedPassword =
        password || contestPasswordStore.getPassword(Number(contestId.value));

      const contestResponse = await ContestService.getContest(
        contestId.value,
        storedPassword || null,
      );

      if (storedPassword && !password) {
        // Verify stored password is still valid
        contestPasswordStore.setPassword(
          Number(contestId.value),
          storedPassword,
        );
      } else if (password) {
        contestPasswordStore.setPassword(Number(contestId.value), password);
      }
      contestData.value = contestResponse;
      document.title = `Submitting to ${problemResponse.name} in ${contestResponse.name} - ${APP_NAME}`;
    } else {
      document.title = `Submitting to ${problemResponse.name} - ${APP_NAME}`;
    }
  } catch (e) {
    handleApiError(e);
  }
};

onMounted(async () => {
  await Promise.all([loadSupportedLanguages(), loadProblemAndContestData()]);
});

const saveSelectedLanguage = () => {
  if (lang.value) {
    localStorage.setItem(STORAGE_KEY_LAST_LANGUAGE, lang.value);
  }
};

const handleSubmit = async () => {
  if (!code.value.trim()) {
    toast.error("Code cannot be empty");
    return;
  }

  isSubmitting.value = true;

  try {
    if (!lang.value) {
      throw new Error("You must select a lang!");
    }
    const requestBody: SubmitRequest = {
      code: code.value,
      lang: lang.value,
      contestId: contestId.value || null,
    };

    const response = await ProblemService.submit(problemId.value, requestBody);

    toast.success("Submission created successfully");

    if (isContestMode.value) {
      if (!contestId.value) {
        toast.error("invalid contest!");
        return;
      }
      router.push(
        buildPath(routeMap.contestSubmission.path, {
          contestId: contestId.value,
          problemId: problemId.value,
          submissionId: response.submissionId,
        }),
      );
      return;
    }
    router.push(
      buildPath(routeMap.submission.path, {
        problemId: problemId.value,
        submissionId: response.submissionId,
      }),
    );
  } catch (error) {
    handleApiError(error);
  } finally {
    isSubmitting.value = false;
  }
};
</script>

<template>
  <div class="container mx-auto max-w-6xl">
    <div class="card bg-base-100 shadow-xl">
      <div class="card-body">
        <div class="mb-4">
          <h2 class="text-2xl font-bold flex flex-wrap items-center gap-2">
            <Icon icon="fa6-solid:code" class="w-6 h-6" />
            <span>Submit Solution</span>
            <template v-if="problemName">
              <span>to</span>
              <RouterLink :to="isContestMode
                ? buildPath(routeMap.contestProblem.path, { contestId: contestId!, problemId: problemId })
                : buildPath(routeMap.problem.path, { id: problemId })" class="link link-primary">
                {{ problemName }}
              </RouterLink>
            </template>
            <template v-if="isContestMode && contestData">
              <span>in</span>
              <RouterLink :to="buildPath(routeMap.contest.path, { id: contestId! })" class="link link-primary">
                {{ contestData.name }}
              </RouterLink>
            </template>
          </h2>
        </div>

        <div class="form-control w-full mb-4">
          <label class="label">
            <span class="label-text font-semibold">Programming Language</span>
          </label>
          <select v-model="lang" class="select select-bordered w-full" @change="saveSelectedLanguage">
            <option v-for="language in languages" :key="language.value" :value="language.value">
              {{ language.label }}
            </option>
          </select>
        </div>

        <div class="form-control w-full mb-4">
          <label class="label">
            <span class="label-text font-semibold">Code</span>
          </label>
          <textarea v-model="code" class="textarea textarea-bordered font-mono h-96"
            placeholder="Write your code here..."></textarea>
        </div>

        <div class="card-actions justify-end">
          <button @click="handleSubmit" class="btn btn-primary" :disabled="isSubmitting">
            <Icon v-if="isSubmitting" icon="fa6-solid:spinner" class="w-5 h-5 animate-spin" />
            <Icon v-else icon="fa6-solid:paper-plane" class="w-5 h-5" />
            {{ isSubmitting ? "Submitting..." : "Submit" }}
          </button>
        </div>
      </div>
    </div>
  </div>
</template>
