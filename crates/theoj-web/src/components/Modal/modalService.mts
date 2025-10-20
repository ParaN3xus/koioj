import { markRaw, shallowRef } from "vue";
import type { Modal, ModalConfig } from "./types.mts";

type SubscriberCallback = (modal: Modal | null) => void;

class ModalService {
  private modal = shallowRef<Modal | null>(null);
  private subscribers: SubscriberCallback[] = [];
  private dialogElement: HTMLDialogElement | null = null;

  subscribe(callback: SubscriberCallback): void {
    this.subscribers.push(callback);
    callback(this.modal.value);
  }

  unsubscribe(callback: SubscriberCallback): void {
    this.subscribers = this.subscribers.filter((sub) => sub !== callback);
  }

  private notify(): void {
    this.subscribers.forEach((callback) => { const _ = callback(this.modal.value) });
  }

  open(config: ModalConfig): void {
    this.modal.value = {
      component: markRaw(config.component),
      attrs: config.attrs || {},
      slots: config.slots || {},
    };

    this.notify();

    setTimeout(() => {
      const dialogs = document.querySelectorAll("#modal-root dialog");
      this.dialogElement = dialogs[0] as HTMLDialogElement;
      if (this.dialogElement?.showModal) {
        this.dialogElement.showModal();
      }
    }, 10);
  }

  close(): void {
    if (this.dialogElement?.close) {
      this.dialogElement.close();

      setTimeout(() => {
        this.modal.value = null;
        this.dialogElement = null;
        this.notify();
      }, 200);
    } else {
      this.modal.value = null;
      this.dialogElement = null;
      this.notify();
    }
  }

  getModal(): Modal | null {
    return this.modal.value;
  }
}

export const modalService = new ModalService();
