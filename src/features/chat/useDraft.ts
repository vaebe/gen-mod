import { shallowRef, watch } from "vue";
import { modService } from "@/services/native";
import { useProjectsStore } from "@/stores/projects";

/** 保存串行化，离开前 flush；失败时保留输入并阻止切换。 */
export function useDraft() {
  const requirement = shallowRef("");
  const error = shallowRef("");
  const saving = shallowRef(false);
  let projectId = "";
  let saved = "";
  let sessionId: string | undefined;
  let pending: Promise<boolean> | undefined;
  const revisions = new Map<string, number>();
  let loading = false;
  watch(requirement, () => {
    if (!loading) {
      const key = projectId + "/" + sessionId;
      revisions.set(key, (revisions.get(key) ?? 0) + 1);
    }
  }, { flush: "sync" });
  const projects = useProjectsStore();
  function load(id: string, value: string, session?: string): void {
    projectId = id;
    sessionId = session;
    saved = value;
    loading = true;
    requirement.value = value;
    loading = false;
    error.value = "";
  }
  async function flush(): Promise<boolean> {
    if (pending) { if (!await pending) { return false; } return flush(); }
    if (!projectId || saved === requirement.value) { return true; }
    const text = requirement.value;
    const id = projectId;
    const targetSession = sessionId;
    saving.value = true;
    const operation = (async () => {
      try {
        projects.upsert(await modService.saveDraft(id, text, targetSession));
        if (id !== projectId || targetSession !== sessionId) { return true; }
        saved = text;
        error.value = "";
        return true;
      } catch (cause) { error.value = (cause as Error).message; return false; }
      finally { saving.value = false; }
    })();
    pending = operation;
    const result = await operation;
    if (pending === operation) { pending = undefined; }
    return result && (saved === requirement.value || await flush());
  }
  function captureSubmitted(): () => Promise<void> {
    const id = projectId;
    const targetSession = sessionId;
    const text = requirement.value;
    const key = id + "/" + targetSession;
    const revision = revisions.get(key) ?? 0;
    return () => clearSubmitted(id, targetSession, text, key, revision);
  }
  async function clearSubmitted(id: string, targetSession: string | undefined, text: string, key: string, revision: number): Promise<void> {
    while (pending) {
      const operation = pending;
      await operation;
      if (pending === operation) { pending = undefined; }
    }
    if (!targetSession || (revisions.get(key) ?? 0) !== revision) { return; }
    if (id === projectId && targetSession === sessionId && requirement.value !== text) { return; }
    saving.value = true;
    const operation = (async () => {
      try {
        projects.upsert(await modService.saveDraft(id, "", targetSession, text));
        if (id === projectId && targetSession === sessionId && requirement.value === text && (revisions.get(key) ?? 0) === revision) {
          saved = "";
          requirement.value = "";
        }
        return true;
      } catch (cause) { error.value = (cause as Error).message; return false; }
      finally { saving.value = false; }
    })();
    pending = operation;
    const result = await operation;
    if (pending === operation) { pending = undefined; }
    if (!result) { throw new Error("已发送，草稿清理失败，请勿重复发送"); }
  }
  return { requirement, error, saving, load, flush, captureSubmitted };
}
