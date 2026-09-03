import { invoke } from "@tauri-apps/api/core";

export type BootstrapStatus = {
  application: string;
  status: string;
};

export function bootstrapStatus(): Promise<BootstrapStatus> {
  return invoke<BootstrapStatus>("bootstrap_status");
}
