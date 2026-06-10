import { invoke, isTauri } from "@tauri-apps/api/core";

const NOT_TAURI_MESSAGE =
  "Este recurso só funciona no app desktop. Feche o navegador e inicie com: npm run tauri:dev";

export function runningInTauri(): boolean {
  return isTauri();
}

export async function tauriInvoke<T>(
  cmd: string,
  args: Record<string, unknown> = {},
): Promise<T> {
  if (!isTauri()) {
    throw new Error(NOT_TAURI_MESSAGE);
  }
  return invoke<T>(cmd, args);
}
