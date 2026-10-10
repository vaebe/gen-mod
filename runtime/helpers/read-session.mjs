import { SessionManager } from "@earendil-works/pi-coding-agent";

// 仅构建已有会话的历史投影，不创建 Agent，不执行模型或工具。
const [file, directory, cwd] = process.argv.slice(2);
if (!file || !directory || !cwd) { throw new Error("缺少会话读取参数"); }
const session = SessionManager.open(file, directory, cwd);
const entries = session.getEntries().filter(entry => entry.type === "message");
const messages = session.buildSessionContext().messages.map(message => {
  const entry = entries.find(entry => entry.message === message);
  return entry ? { ...message, appEntryId: entry.id } : message;
});
process.stdout.write(JSON.stringify({ messages }));
