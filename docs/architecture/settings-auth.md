# 设置与保留认证能力

`features/settings` 拥有设置页面、服务商与模型编辑器、配置状态 composable 和模型类型。页面与聊天消费者通过 `configurationService` 读取、保存和打开配置目录，不自行声明 IPC 返回类型。会话模型切换仍归会话通信模块，其选择与模型状态复用设置类型。

| 业务操作 | 原生命令 | 参数 | 结果 |
| --- | --- | --- | --- |
| 读取配置 | get_configuration | 无 | ConfigurationStatus |
| 保存配置 | save_configuration | configuration: SaveConfiguration | ConfigurationStatus |
| 打开配置目录 | open_configuration_directory | 无 | void |
| 登录/注册 | cloud_login / cloud_register | email、password | CloudUser |
| 当前用户 | cloud_me | 无 | CloudUser |
| 退出登录 | cloud_logout | 无 | void |

`features/auth` 保留登录界面、展示状态及 `authService`，包括暂时没有 UI 消费者的当前用户查询。登录和注册继续发出 authenticated 事件；退出成功后才发出现有认证失效通知。云端 Skills 更新仍归 `features/skills`，仅退出操作调用认证服务。

上述命令失败继续由 `nativeCall` 将 Rust 字符串错误转为 Error；`CLOUD_AUTH_INVALID:` 仍触发 genmod-auth-invalid。配置保存失败保留草稿，原生配置锁、校验、写入失败恢复、认证刷新与凭据清理规则不变。读取配置不返回已保存 Key；登录返回用户展示信息，令牌保持在原生层，不增加凭据日志或前端存储。
