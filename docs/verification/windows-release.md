# Windows 分发与验收记录

2026-10-09。状态：前端正式构建及 Windows x64 MSI/NSIS 构建通过，打包后 release 资源一致性检查通过。未安装、解包或运行安装程序，未运行应用或自动化测试。下表未执行场景不是通过记录。

## 已有静态证据

- 前端限定范围类型检查、三个 pi extension 类型检查与原生库检查通过，详见 implementation-progress.md。
- 固定 Node 22.19.0 Windows x64 + pi 1.1.0；prepare-runtime 生成 17,374 个运行时文件和 13 个 skills 文件的哈希清单，运行时模块为实体目录。
- bundle.resources 与开发/发布路径一致，只包含固定运行时、helpers/extensions、内置 skills 和哈希清单。应用 env、用户源码/日志和游戏参考程序集未列入资源。
- 开发模型配置读取项目根 env，发布读取 appConfigDir/.env；设置页返回脱敏状态。真实 Key 在最终验收时由用户补充。

## 待执行清单

| 场景 | 操作与预期 | 状态 |
| --- | --- | --- |
| A：项目/布局 | 无 Key 新建与重开项目，重命名不改变目录；草稿保存；1280×800 与窄窗口检查导航、Sheet、输入区 | 未执行 |
| B：RPC/历史 | 内置运行时启动；工具/正文分层；取消正常结束，超时可重启并回收后代；重开历史不重发需求；旧上下文只读 | 未执行 |
| B：文件变更 | 在隔离 source 夹具检查新增/修改/删除、命令改文件、二进制摘要及历史差异；用户上滚停止跟随 | 未执行 |
| Skills | 导入/禁用/依赖缺失与环可见；仅所选游戏闭包加载，get_commands 与界面一致；内容改变后新会话 | 未执行 |
| C：版本登记 | 通用纯文本夹具交付 A/B，A 不改变；旧任务、未知/失败命令、越界和不一致 ZIP 不登记；重复事件返回同一版本 | 未执行 |
| Duckov 缺依赖 | 保留 C# 模板，prepare 报告具体缺 SDK/DLL；不扫描游戏、不伪造 DLL | 未执行 |
| Duckov 有依赖 | 用户提供有效参考目录后，执行真实 build/package，交付仅含 MOD 文件；统一名称/入口 | 未执行 |
| D：安装成功 | 在隔离临时目录预览完整映射/覆盖；安装 A、生成 B 显示待安装；选定 B 安装成功才更新关联；导出仍使用选定版本 | 未执行 |
| D：安装失败 | 复制中途失败恢复备份并移除新增文件；回退失败显示 uncertain；预览后文件改变要求重新确认 | 未执行 |
| 安装包构建 | 执行计划内 pnpm build、Tauri build，记录实际命令、文件路径及结果；不签名/发布 | 前端与 Windows MSI/NSIS 构建通过；未安装/运行 |
| 无全局依赖环境 | 安装包在无全局 Node/pi 环境启动；中文/空格目录；env、skills、历史与临时目录安装完整 | 未执行 |
| E：真实 Responses | 用户补 Key 后验证指定 StepFun 文本流、一次文件工具调用、需求到 ZIP/版本登记与临时目标按钮安装，核对任务/命令/版本 | 等待用户补 Key并执行验收 |
| E：错误展示 | 用户确认的无效测试值验证认证错误；隔离配置验证连接错误后恢复；不自动切协议或泄露 Key | 未执行 |

## 构建产物

项目根目录：`D:\codetest\gen-mod`。以下路径相对此目录，两个安装程序均未签名、安装或发布。

| 产物 | 字节数 | SHA-256 |
| --- | ---: | --- |
| `src-tauri/target/release/bundle/msi/gen-mod_0.1.0_x64_en-US.msi` | 76,275,768 | `484A3C836069A6ED72B4C3E466E30A2E775A7958EA62A4941B0D9CFEB387911D` |
| `src-tauri/target/release/bundle/nsis/gen-mod_0.1.0_x64-setup.exe` | 43,930,473 | `0ADA2E2CA5485E97CD24C837649665A3140477103171F277AB911B5F4B0A312E` |

实际命令为 `node node_modules/pnpm/bin/pnpm.cjs build` 和 `node node_modules/@tauri-apps/cli/tauri.js build`，均退出 0。前端保留 ChatPage 约 693 kB 的分块警告。首次 Tauri 构建因 skill 资源更新而主动停止旧资源 WiX 进程，退出 1；上述产物来自后续完整成功构建。

打包完成后 `node work/check-release-resources.mjs` 通过：release 清单与当前准备清单一致，17,374 个运行时文件与 13 个 skills 文件的大小及 SHA-256 均匹配。`Get-FileHash -Algorithm SHA256` 记录上述安装程序校验值；资源检查没有解包安装程序，不能替代安装与运行验收。

API Key 缺失不作为实现卡点。安装验收不默认写入真实游戏目录；游戏内启用、兼容性和效果验证由用户负责。
