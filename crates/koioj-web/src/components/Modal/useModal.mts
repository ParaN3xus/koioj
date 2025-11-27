import { modalService } from './modalService.mts';
import type { ModalConfig, UseModalReturn } from './types.mts';

export function useModal(config: ModalConfig): UseModalReturn {
  const open = (): void => {
    modalService.open(config);
  };

  const close = (): void => {
    modalService.close();
  };

  return { open, close };
}
