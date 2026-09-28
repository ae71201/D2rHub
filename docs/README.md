# 文档目录

用户从使用手册开始，开发者从开发指南和架构说明开始。历史记录保留决策背景，不作为当前操作说明。

## 使用

- [使用手册](user-guide.html)：分步上手、Mod、扩展功能和故障排查；与桌面应用共用同一离线页面。
- [更新记录](../CHANGELOG.md)：版本变化与历史记录。
- [安全政策](../SECURITY.md)：权限、本地数据、联网范围与漏洞报告。

## 开发与维护

- [开发指南](DEVELOPMENT.md)：环境、运行、验证和目录归属。
- [架构说明](architecture.md)：当前结构、依赖边界与维护约定。
- [贡献指南](../CONTRIBUTING.md)：提交流程与兼容要求。
- [发布指南](release-workflow.md)：唯一维护者入口，负责准备、发布与补传。
- [双源更新协议](dual-source-updates.md)：清单、兼容、下载与事务边界。
- [资源分发](mod-resource-distribution.md)：软件、加工器和 Mod 的归属。
- [音频遥测 v7](AUDIO_TELEMETRY_V7.md)：Hub 端当前解码协议。
- [轻量 Mod](lightweight-mod-generation.md)、[桌宠衣柜](PET_WARDROBE.md)：领域专项说明。

## 决策与验证记录

- [ADR](adr/)：双客户端启动上下文、模块架构和纯净模式。
- [登录就绪研究](research/d2r-token-readiness.md)与[实机验证计划](research/loader-readiness-test-plan.md)。
- [重构验收记录](REFACTOR_ACCEPTANCE.md)、[性能基线](release-performance-baseline.json)。
- [本轮审查与实施](review-2026-09-28.md)。

以下是有日期或阶段边界的历史证据，不能代替当前实现与验证：

- [2026-08-31 代码审查](code-review-2026-08-31.md)。
- [音频遥测 v4](AUDIO_TELEMETRY_V4.md)：旧协议，当前使用 v7。
- [桌宠开发记录](pet-progress.md)与[桌宠视觉验收](pet-visual-review/README.md)。
- [手册截图来源](guide-images/README.md)：包含截图日期、模拟状态和更新限制。

## 文档维护约定

README 只回答产品用途、下载安装、开始使用和开发入口。操作细节写入手册，版本变化写入 `releases/` 并从更新记录链接；架构事实写入架构说明，决策过程写入 ADR。不要在多个入口重复维护完整功能列表或历史更新日志。

`user-guide.html` 和 `stats.html` 是桌面运行资源，修改路径必须同步 Tauri 资源清单与 Rust 读取路径。`guide-images/` 是手册运行资源；其他审查图和开发记录不打入安装包。`resources/` 中的 JSON 属于版本化分发协议；v1 保留兼容，新的发布使用 v2。
