# 玩家反馈能力

`src/features/verification/` 拥有反馈契约 `types.ts`、读取与项目失效状态 `useVerification.ts`、草稿编辑/确认/拒绝界面 `VerificationCard.vue` 和确认历史 `VerificationHistory.vue`。聊天与交付页面组合这些入口，不直接声明反馈 IPC 的返回类型。

`verificationService.ts` 绑定 `list_verification`、`edit_feedback_draft`、`confirm_feedback_draft`、`reject_feedback_draft` 的参数及结果；通用 `nativeCall` 保留错误转换。原生所有者仍是 `src-tauri/src/verification/`，负责可写检查、草稿 revision、证据校验和确认事务。运行时工具反馈请求的校验边界不变。

后台反馈事件由 `src/application/projectRefresh.ts` 根据事件原 `modId` 交给既有 `invalidateVerification`。卡片写入成功使用发起时项目身份失效；读取按项目、版本和请求代次过滤过期结果，并在视图销毁时失效。保留本地编辑冲突提示和先保存后确认规则，不额外建立刷新通道。

相关既有回归测试在 `src/features/sessions/sessionRuntime.test.ts`，覆盖后台原项目失效及同项目视图同步，仍由 `vitest.config.ts` 的 sessions 模式发现。本次契约归属整理不移动测试，不新增目录镜像断言；卡片真实交互及原生写入需分别验收。
