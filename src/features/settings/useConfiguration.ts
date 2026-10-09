import { shallowRef } from "vue";
import type { ConfigurationStatus } from "@/contracts/config";
import { nativeCall } from "@/services/native";

/** 只接收脱敏状态，不持有或编辑 API Key。 */
export function useConfiguration() {
  const status = shallowRef<ConfigurationStatus>();
  const error = shallowRef("");
  const loading = shallowRef(false);
  async function refresh(): Promise<void> {
    loading.value = true;
    error.value = "";
    try { status.value = await nativeCall<ConfigurationStatus>("get_configuration"); }
    catch (cause) { error.value = (cause as Error).message; }
    finally { loading.value = false; }
  }
  async function openDirectory(): Promise<void> {
    try { await nativeCall("open_configuration_directory"); }
    catch (cause) { error.value = (cause as Error).message; }
  }
  return { status, error, loading, refresh, openDirectory };
}
