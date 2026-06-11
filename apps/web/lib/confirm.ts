export type ConfirmTone = "danger" | "default";

export type ConfirmRequest = {
  id: number;
  title: string;
  message: string;
  confirmLabel: string;
  cancelLabel: string;
  tone: ConfirmTone;
  resolve: (confirmed: boolean) => void;
};

type ConfirmListener = (request: ConfirmRequest) => void;

let nextId = 1;
const listeners = new Set<ConfirmListener>();

export function subscribeToConfirms(listener: ConfirmListener): () => void {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

export function confirmAction(options: {
  title: string;
  message: string;
  confirmLabel?: string;
  cancelLabel?: string;
  tone?: ConfirmTone;
}): Promise<boolean> {
  if (typeof window === "undefined") return Promise.resolve(false);

  return new Promise((resolve) => {
    const request: ConfirmRequest = {
      id: nextId++,
      title: options.title,
      message: options.message,
      confirmLabel: options.confirmLabel ?? "Confirm",
      cancelLabel: options.cancelLabel ?? "Cancel",
      tone: options.tone ?? "default",
      resolve,
    };
    listeners.forEach((listener) => listener(request));
  });
}
