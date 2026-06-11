export type Toast = {
  id: number;
  tone: "error" | "success" | "info";
  title?: string;
  message: string;
};

type ToastListener = (toast: Toast) => void;

let nextId = 1;
const listeners = new Set<ToastListener>();

export function subscribeToToasts(listener: ToastListener): () => void {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

export function showToast(message: string, tone: Toast["tone"] = "info", title?: string) {
  if (typeof window === "undefined") return;
  const toast: Toast = { id: nextId++, tone, title, message };
  listeners.forEach((listener) => listener(toast));
}
