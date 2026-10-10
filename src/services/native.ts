import { invoke } from "@tauri-apps/api/core";
import type { ModProject } from "../contracts/mod";

/** 原生错误统一为可展示的 Error，调用方负责保留用户输入。 */
export async function nativeCall<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(command, args);
  } catch (error) {
    throw new Error(typeof error === "string" ? error : String(error));
  }
}

export const modService = {
  create(name: string, gameSkillName: string, requirement: string): Promise<ModProject> {
    return nativeCall("create_mod", { name, gameSkillName, requirement });
  },
  list(): Promise<ModProject[]> {
    return nativeCall("list_mods");
  },
  get(modId: string): Promise<ModProject> {
    return nativeCall("get_mod", { modId });
  },
  rename(modId: string, name: string): Promise<ModProject> {
    return nativeCall("rename_mod", { modId, name });
  },
  saveDraft(modId: string, requirement: string, sessionId?: string, expectedRequirement?: string): Promise<ModProject> {
    return nativeCall("save_mod_draft", { modId, requirement, sessionId, expectedRequirement });
  },
};
