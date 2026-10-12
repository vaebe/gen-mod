import { shallowRef } from "vue";
import type { ConfigurationStatus, SaveConfiguration } from "@/contracts/config";
import { nativeCall } from "@/services/native";

// 读取脱敏状态；新输入的 Key 只通过保存请求传给原生层。
export function useConfiguration() {
  const status = shallowRef<ConfigurationStatus>();
  const error = shallowRef("");
  const loading = shallowRef(false);
  const saving = shallowRef(false);
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
  async function save(configuration: SaveConfiguration): Promise<boolean> {
    if (saving.value) { return false; }
    saving.value = true;
    error.value = "";
    try {
      status.value = await nativeCall<ConfigurationStatus>("save_configuration", { configuration });
      return true;
    } catch (cause) { error.value = (cause as Error).message; return false; }
    finally { saving.value = false; }
  }
  return { status, error, loading, saving, refresh, openDirectory, save };
}
