# Harmony 与兼容性

## 何时使用

公开事件或 API 可满足需求时直接使用。必须改变原有方法行为时，再考虑 Harmony。上游 Duckov README 提示游戏未内置 Harmony，并观察到多版本加载冲突；其 2.4.1 表述是写作时示例，不是永久版本要求。

先核对用户现有前置、Harmony 程序集身份和版本。新增前置、安装包或交付第三方运行库须获得明确授权；编译引用存在不代表运行时已加载。Harmony 通用文档对多版本共存的说明不保证 Duckov 的组合兼容性，应优先核对游戏社区实际约束。

## 目标与补丁选择

1. 确认目标类型、重载参数和当前方法实现，追踪它在用户场景中的调用位置；只命中同名方法可能补错重载。
2. 优先选择能表达需求的简单补丁。Postfix 适合读取或调整结果；Prefix 适合处理调用前参数，返回 false 跳过原方法会改变原有副作用，必须符合需求。
3. Patch 方法是 static；__instance 表示原实例，__result 表示结果，改写结果时核对 ref 和实际类型。跨 Prefix/Postfix 的 __state 及参数注入按当前 Harmony 版本文档确认。
4. Transpiler 仅在必须改 IL 且简单补丁无法完成时使用；核对匹配指令与版本，不因“找不到字段”直接升级成 IL 修改。
5. 需要处理原方法异常时确认 Finalizer 语义；不为让功能看似正常而吞掉未知异常。

来源：[Patching](https://harmony.pardeike.net/v2/articles/patching.html)、[参数注入](https://harmony.pardeike.net/v2/articles/patching-injections.html)。

## 注册与释放

为当前 Mod 使用唯一、稳定的 Harmony owner ID，例如自有反向域名风格标识，不能与别的 Mod 共用。PatchAll 指定当前 Mod 程序集，防止扫描范围意外扩大；保证启用时只注册一次。

退出或禁用时只移除自己 owner ID 下的补丁。使用当前版本支持的撤销 API；如果调用 UnpatchAll，必须显式传入自己的 ID，不调用无参数形式删除其他 Mod 的补丁。需要重新启用的 Mod 在清理后恢复自身注册状态。

来源：[Harmony Basics](https://harmony.pardeike.net/v2/articles/basics.html)。

## 兼容性排查

- 方法重载、可见性、签名或返回类型变化：重新核对当前游戏 DLL，不用候选字符串循环猜测。
- 多个补丁影响同一方法：使用 GetPatchInfo 等当前版本提供的查询核对 owner；只有确实依赖执行顺序时才设置 before/after 或 priority。
- Prefix 跳过原方法：检查其他补丁与原方法的状态初始化、奖励、存档等副作用是否被跳过。
- 运行库加载异常：区分缺少前置、加载顺序与版本不一致，提供实际异常；不把全部错误归为“Mod 冲突”。

排查阶段记录已确认目标与未验证组合。没有用户日志或游戏内验证时，只能说明静态推导，不能声称已复现冲突。
