import type { Component, VNode } from "vue";

export interface ModalSlots {
  default?: string | VNode;
  [key: string]: string | VNode | undefined;
}

export interface ModalAttrs {
  [key: string]: unknown;
}

export interface ModalConfig {
  component: Component;
  attrs?: ModalAttrs;
  slots?: ModalSlots;
}

export interface Modal {
  component: Component;
  attrs: ModalAttrs;
  slots: ModalSlots;
}

export interface UseModalReturn {
  open: () => void;
  close: () => void;
}
