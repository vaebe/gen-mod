import { defineStore } from "pinia";
import { shallowRef } from "vue";
import type { ModProject } from "@/contracts/mod";
import { modService } from "@/services/native";

/** 跨页面共享项目列表，读取失败不能伪装成空项目。 */
export const useProjectsStore = defineStore("projects", () => {
  const projects = shallowRef<ModProject[]>([]);
  const error = shallowRef("");
  const loading = shallowRef(false);
  async function refresh(): Promise<void> {
    loading.value = true;
    error.value = "";
    try { projects.value = await modService.list(); }
    catch (cause) { error.value = (cause as Error).message; }
    finally { loading.value = false; }
  }
  function upsert(project: ModProject): void {
    projects.value = [project, ...projects.value.filter((item) => item.modId !== project.modId)];
  }
  return { projects, error, loading, refresh, upsert };
});
