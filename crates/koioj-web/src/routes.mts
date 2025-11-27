import type { RouteComponent, RouteRecordRaw } from "vue-router";
import Contest from "./pages/Contest.vue";
import ContestList from "./pages/ContestList.vue";
import CreateSolution from "./pages/CreateSolution.vue";
import EditContest from "./pages/EditContest.vue";
import EditProblem from "./pages/EditProblem.vue";
import EditTrainingPlan from "./pages/EditTrainingPlan.vue";
import Index from "./pages/Index.vue";
import Login from "./pages/Login.vue";
import NotFound from "./pages/NotFound.vue";
import OverallRanking from "./pages/OverallRanking.vue";
import Problem from "./pages/Problem.vue";
import ProblemList from "./pages/ProblemList.vue";
import Profile from "./pages/Profile.vue";
import Register from "./pages/Register.vue";
import Solution from "./pages/Solution.vue";
import SolutionList from "./pages/SolutionList.vue";
import Submission from "./pages/Submission.vue";
import Submit from "./pages/Submit.vue";
import TrainingPlan from "./pages/TrainingPlan.vue";
import TrainingPlanList from "./pages/TrainingPlanList.vue";
import { APP_NAME } from "./utils.mts";

const createRoutes = <
  T extends Record<
    string,
    { path: string; component: RouteComponent; title: string }
  >,
>(
  routeConfigs: T,
) => {
  const routes = {} as Record<keyof T, RouteRecordRaw>;

  Object.entries(routeConfigs).forEach(([name, { path, component, title }]) => {
    routes[name as keyof T] = {
      path,
      name,
      component,
      meta: { title },
    };
  });

  return routes;
};

export const routeMap = createRoutes({
  index: {
    path: "/",
    component: Index,
    title: `${APP_NAME} - The Online Judge Platform`,
  },
  login: {
    path: "/users/login",
    component: Login,
    title: `Login - ${APP_NAME}`,
  },
  register: {
    path: "/users/register",
    component: Register,
    title: `Register - ${APP_NAME}`,
  },
  profile: {
    path: "/users/profile/:id",
    component: Profile,
    title: `Profile - ${APP_NAME}`,
  },
  problemList: {
    path: "/problem",
    component: ProblemList,
    title: `Problem - ${APP_NAME}`,
  },
  createProblem: {
    path: "/problem/new",
    component: EditProblem,
    title: `New Problem - ${APP_NAME}`,
  },
  problem: {
    path: "/problem/:id",
    component: Problem,
    title: `Problem - ${APP_NAME}`,
  },
  editProblem: {
    path: "/problem/:id/edit",
    component: EditProblem,
    title: `Edit Problem - ${APP_NAME}`,
  },
  soloutionList: {
    path: "/problem/:id/solution",
    component: SolutionList,
    title: `Solution - ${APP_NAME}`,
  },
  createSolution: {
    path: "/problem/:id/solution/new",
    component: CreateSolution,
    title: `New Solution - ${APP_NAME}`,
  },
  solution: {
    path: "/problem/:problemId/solution/:solutionId",
    component: Solution,
    title: `Solution - ${APP_NAME}`,
  },
  submit: {
    path: "/problem/:id/submit",
    component: Submit,
    title: `Submit - ${APP_NAME}`,
  },
  submission: {
    path: "/problem/:problemId/submission/:submissionId",
    component: Submission,
    title: `Submission - ${APP_NAME}`,
  },
  contestList: {
    path: "/contest",
    component: ContestList,
    title: `Contest - ${APP_NAME}`,
  },
  createContest: {
    path: "/contest/new",
    component: EditContest,
    title: `New Contest - ${APP_NAME}`,
  },
  contest: {
    path: "/contest/:id",
    component: Contest,
    title: `Contest - ${APP_NAME}`,
  },
  editContest: {
    path: "/contest/:id/edit",
    component: EditContest,
    title: `Edit Contest - ${APP_NAME}`,
  },
  contestProblem: {
    path: "/contest/:contestId/problem/:problemId",
    component: Problem,
    title: `Problem - ${APP_NAME}`,
  },
  contestSubmit: {
    path: "/contest/:contestId/problem/:problemId/submit",
    component: Submit,
    title: `Submit - ${APP_NAME}`,
  },
  contestSubmission: {
    path: "/contest/:contestId/problem/:problemId/submission/:submissionId",
    component: Submission,
    title: `Submission - ${APP_NAME}`,
  },
  overallRanking: {
    path: "/contest/overall-ranking",
    component: OverallRanking,
    title: `OverallRanking - ${APP_NAME}`,
  },
  trainingPlanList: {
    path: "/training",
    component: TrainingPlanList,
    title: `Training Plan - ${APP_NAME}`,
  },
  createTrainingPlan: {
    path: "/training/new",
    component: EditTrainingPlan,
    title: `New Training Plan - ${APP_NAME}`,
  },
  editTrainingPlan: {
    path: "/training/:id/edit",
    component: EditTrainingPlan,
    title: `Edit Training Plan - ${APP_NAME}`,
  },
  trainingPlan: {
    path: "/training/:id",
    component: TrainingPlan,
    title: `Training Plan - ${APP_NAME}`,
  },
  notFound: {
    path: "/:pathMatch(.*)*",
    component: NotFound,
    title: `404 Not Found - ${APP_NAME}`,
  },
});

export const routes: RouteRecordRaw[] = Object.values(routeMap);

export function buildPath(
  base: string,
  params: Record<string, string | number>,
): string {
  let path = base;

  for (const [key, value] of Object.entries(params)) {
    path = path.replace(`:${key}`, String(value));
  }

  return path;
}
