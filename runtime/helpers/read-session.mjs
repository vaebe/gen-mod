import { SessionManager } from "@earendil-works/pi-coding-agent";

// 仅构建已有会话的历史投影，不创建 Agent，不执行模型或工具。
import { createHash } from "node:crypto";
const [file, directory, cwd, mode] = process.argv.slice(2);
if (!file || !directory || !cwd) { throw new Error("缺少会话读取参数"); }
const session = SessionManager.open(file, directory, cwd);
const entries = session.getEntries().filter(entry => entry.type === "message");
const messages = session.buildSessionContext().messages.map(message => {
  const entry = entries.find(entry => entry.message === message);
  return entry ? { ...message, appEntryId: entry.id } : message;
});
function canonical(value) {
  if (Array.isArray(value)) { return value.map(canonical); }
  if (value && typeof value === "object") { return Object.fromEntries(Object.keys(value).sort().map(key => [key, canonical(value[key])])); }
  return value;
}
const digest = value => createHash("sha256").update(JSON.stringify(canonical(value))).digest("hex");
if (mode === "evidence") {
  const sources = entries.filter(entry => entry.message.role === "user").slice(-50).map(entry => ({
    entryId: entry.id, sha256: digest(entry.message),
    text: (typeof entry.message.content === "string" ? entry.message.content : entry.message.content.filter(item => item.type === "text").map(item => item.text).join("\n")).slice(0, 2000),
    attachments: Array.isArray(entry.message.content) ? entry.message.content.flatMap((part, index) => part.type === "image" ? [{ index, sha256: digest(part), mimeType: part.mimeType }] : []) : [],
  }));
  process.stdout.write(JSON.stringify({ sources }));
} else { process.stdout.write(JSON.stringify({ messages })); }
