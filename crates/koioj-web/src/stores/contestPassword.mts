import { defineStore } from 'pinia';
import { ref } from 'vue';

interface ContestPasswords {
  [contestId: number]: string;
}

export const useContestPasswordStore = defineStore('contestPassword', () => {
  const passwords = ref<ContestPasswords>({});

  // init from localStoreage
  const loadFromStorage = () => {
    try {
      const stored = localStorage.getItem('contest_passwords');
      if (stored) {
        passwords.value = JSON.parse(stored);
      }
    } catch (e) {
      console.error('Failed to load contest passwords:', e);
    }
  };

  // save
  const saveToStorage = () => {
    try {
      localStorage.setItem('contest_passwords', JSON.stringify(passwords.value));
    } catch (e) {
      console.error('Failed to save contest passwords:', e);
    }
  };

  const getPassword = (contestId: number): string | undefined => {
    return passwords.value[contestId];
  };

  const setPassword = (contestId: number, password: string) => {
    passwords.value[contestId] = password;
    saveToStorage();
  };

  const clearPassword = (contestId: number) => {
    delete passwords.value[contestId];
    saveToStorage();
  };

  const clearAllPasswords = () => {
    passwords.value = {};
    saveToStorage();
  };

  loadFromStorage();

  return {
    passwords,
    getPassword,
    setPassword,
    clearPassword,
    clearAllPasswords,
  };
});
