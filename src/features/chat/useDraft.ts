import { shallowRef } from "vue";
import { modService } from "@/services/native";
import { useProjectsStore } from "@/stores/projects";

/** 保存串行化，离开前 flush；失败时保留输入并阻止切换。 */
export function useDraft() {
  const requirement = shallowRef("");
  const error = shallowRef("");
  const saving = shallowRef(false);
  let projectId = "";
  let saved = "";
  let pending: Promise<boolean> | undefined;
  const projects = useProjectsStore();
  function load(id: string, value: string): void {
    projectId = id;
    saved = value;
    requirement.value = value;
    error.value = "";
  }
  async function flush(): Promise<boolean> {
    if (pending) { if (!await pending) { return false; } return flush(); }
    if (!projectId || saved === requirement.value) { return true; }
    const text = requirement.value;
    const id = projectId;
    saving.value = true;
    pending = (async () => {
      try {
        projects.upsert(await modService.saveDraft(id, text));
        saved = text;
        error.value = "";
        return true;
      } catch (cause) { error.value = (cause as Error).message; return false; }
      finally { saving.value = false; }
    })();
    const result = await pending;
    pending = undefined;
    return result && (saved === requirement.value || await flush());
  }
  return { requirement, error, saving, load, flush };
}
