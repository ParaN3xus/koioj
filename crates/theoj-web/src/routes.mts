import type { RouteComponent, RouteRecordRaw } from "vue-router";
import CreateSolution from "./pages/CreateSolution.vue";
import EditProblem from "./pages/EditProblem.vue";
import Index from "./pages/Index.vue";
import Login from "./pages/Login.vue";
import NotFound from "./pages/NotFound.vue";
import Problem from "./pages/Problem.vue";
import ProblemList from "./pages/ProblemList.vue";
import Profile from "./pages/Profile.vue";
import Register from "./pages/Register.vue";
import Solution from "./pages/Solution.vue";
import SolutionList from "./pages/SolutionList.vue";

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
    title: "TheOJ - The Online Judge Platform",
  },
  login: {
    path: "/users/login",
    component: Login,
    title: "Login - TheOJ",
  },
  register: {
    path: "/users/register",
    component: Register,
    title: "Register - TheOJ",
  },
  profile: {
    path: "/users/profile/:id",
    component: Profile,
    title: "Profile - TheOJ",
  },
  problemList: {
    path: "/problem",
    component: ProblemList,
    title: "Problem - TheOJ",
  },
  createProblem: {
    path: "/problem/new",
    component: EditProblem,
    title: "New Problem - TheOJ",
  },
  problem: {
    path: "/problem/:id",
    component: Problem,
    title: "Problem - TheOJ",
  },
  editProblem: {
    path: "/problem/:id/edit",
    component: EditProblem,
    title: "Edit Problem - TheOJ",
  },
  soloutionList: {
    path: "/problem/:id/solution",
    component: SolutionList,
    title: "Solution - TheOJ",
  },
  createSolution: {
    path: "/problem/:id/solution/new",
    component: CreateSolution,
    title: "New Solution - TheOJ",
  },
  solution: {
    path: "/problem/:problemId/solution/:solutionId",
    component: Solution,
    title: "Solution - TheOJ",
  },
  notFound: {
    path: "/:pathMatch(.*)*",
    component: NotFound,
    title: "404 Not Found - TheOJ",
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
