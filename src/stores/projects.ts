import { defineStore } from "pinia";
import { shallowRef } from "vue";
import type { ModProject } from "@/contracts/mod";
import { modService, nativeCall } from "@/services/native";

/** 跨页面共享项目列表，读取失败不能伪装成空项目。 */
export const useProjectsStore = defineStore("projects", () => {
  const projects = shallowRef<ModProject[]>([]);
  const error = shallowRef("");
  const loading = shallowRef(false);
  async function refresh(): Promise<void> {
    loading.value = true;
    error.value = "";
    try {
      const [values, failures, recoveries] = await Promise.all([modService.list(), nativeCall<{modId: string;error: string}[]>("list_migration_errors"),nativeCall<{modId: string;kind: string}[]>("list_recovery_issues")]);
      projects.value = values;
      if (recoveries.length) { error.value = recoveries.map(item => "文件操作需核对 " + item.modId + "：" + item.kind).join("；"); }
      if (failures.length) { error.value = failures.map(item => "迁移失败 " + item.modId + "：" + item.error).join("；"); }
    }
    catch (cause) { error.value = (cause as Error).message; }
    finally { loading.value = false; }
  }
  function upsert(project: ModProject): void {
    projects.value = [project, ...projects.value.filter((item) => item.modId !== project.modId)];
  }
  return { projects, error, loading, refresh, upsert };
});
