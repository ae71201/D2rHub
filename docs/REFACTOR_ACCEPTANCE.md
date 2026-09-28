# 模块化单体重构验收记录

本记录是架构变更的验收清单。最近一次执行结果见 [2026-09-28 整理记录](review-2026-09-28.md)，各项边界的当前归属见 [架构说明](architecture.md)。D2RHub 是编译期组合、统一发布的 Windows
模块化单体，不提供第三方 SDK，不动态加载 DLL，也不拆分微服务。

## 自动验收

| 阶段 | 已建立的可验证边界 |
| --- | --- |
| 1 基线 | 前端、Rust、Release 构建和无副作用启动冒烟；真实游戏验收单独列为人工门禁 |
| 2 后端分层 | `domain`、`application`、`infrastructure`、`commands` 分层；系统命令只做 IPC 转换，Windows 实现位于基础设施 |
| 3 核心模型 | 账号、实例、启动方案、取消代次、账号/目录/宿主租约由多开核心统一管理 |
| 4 能力协议 | 静态能力清单声明 ID、版本、分类、依赖、配置版本、设置入口、命令、事件、生命周期和健康状态 |
| 5 配置兼容 | v0-v9 样本迁移、未知字段保留、CAS、staging/backup、失败恢复和跨资源 journal 测试 |
| 6 前端架构 | Tauri 原始 API 仅存在于平台网关；设置面板和控制器按功能拆分；壳组件设有 900 行回归上限 |
| 7 设置中心 | 游戏 / 扩展 / 应用导航、可用性和能力状态由注册表组合；草稿会话和 Mod 工作流有独立控制器 |
| 8 任务运行时 | 初始化、启动、Mod 加工和自动跟房共享任务状态、冲突键、取消、时间线、错误码和后端重试 |
| 9 诊断 | 结构化任务时间线、能力健康和脱敏日志可导出 ZIP；测试验证路径、账号和凭据不会泄漏 |
| 10 资源治理 | 可选窗口按需创建并在停用时销毁；worker、监听器和快捷键由能力生命周期回收；提供 Release 基线脚本 |
| 11 测试体系 | Rust 单元/迁移/集成测试、前端逻辑/交互/同步测试、架构契约、严格 Clippy 和生产构建 |
| 12 发布收口 | 发布入口从干净提交构建 NSIS，保存源码和检查证据；本地也可生成 MSI，补传复用原产物 |

每次 RC 必须依次执行：

```powershell
npm run check
npm run build
cargo test --locked --manifest-path src-tauri/Cargo.toml
cargo clippy --locked --manifest-path src-tauri/Cargo.toml --all-targets --all-features -- -D warnings
npm run build:nsis -- -- --locked
powershell -NoProfile -ExecutionPolicy Bypass -File scripts/measure-release-baseline.ps1
```

正式发布使用 [统一发布入口](release-workflow.md)；它会在隔离源码快照中重新执行检查并记录结果。上面的工作区构建用于本地验收，不能作为已绑定源码提交的正式发布任务。

## 人工实机门禁

下列行为不能由仓库测试或无副作用冒烟替代，必须在真实 D2R、Battle.net、账号和
本机游戏环境中完成。它们不是重构遗留代码，而是 RC 发布前的外部系统验收：

- CN/Global、Token/Battle.net、多账号串行启动、停止和窗口切换；
- 自动跟房的主号、跟随号、取消和失败重试；
- 音频遥测、识别 Mod 安装/升级、掉落与场景统计；
- 存档目录监听、覆盖层定位、多显示器恢复和全局快捷键；
- 从真实旧版数据升级、失败恢复备份，以及安装包覆盖升级和回滚。

人工门禁失败时不得发布；修复必须增加能够稳定复现该缺陷的自动测试，然后重新执行
全部自动验收。
