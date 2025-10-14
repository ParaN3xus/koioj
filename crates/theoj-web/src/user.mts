import { defineStore } from "pinia";
import { type LoginResponse, OpenAPI, type RegisterResponse, UserService } from "./theoj-api";

export const useUserStore = defineStore("user", {
  state: () => ({
    token: localStorage.getItem("token") || "",
    userId: localStorage.getItem("userId") || "",
  }),

  getters: {
    isLoggedIn: (state): boolean => !!state.token,
  },

  actions: {
    async login(identifier: string, password: string): Promise<void> {
      const res = UserService.login({
        identifier: identifier,
        password: password,
      });
      const data: LoginResponse = await res;

      this.token = data.token;
      this.userId = data.userId;

      localStorage.setItem("token", data.token);
      localStorage.setItem("userId", data.userId);

      OpenAPI.TOKEN = data.token;
    },

    async register(
      email: string,
      phone: string,
      username: string,
      userCode: string,
      password: string,
    ): Promise<void> {
      const res = UserService.register({
        email: email,
        phone: phone,
        username: username,
        userCode: userCode,
        password: password,
      });
      const data: RegisterResponse = await res;

      this.token = data.token;
      this.userId = data.userId;

      localStorage.setItem("token", data.token);
      localStorage.setItem("userId", data.userId);
    },

    logout(): void {
      this.token = "";
      this.userId = "";
      localStorage.removeItem("token");
      localStorage.removeItem("userId");
    },
  },
});
