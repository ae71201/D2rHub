<div align="center">
  <img src="public/logo.png" alt="D2RHub" width="96" />
  <h1>D2RHub</h1>
  <p>暗黑破坏神 II：重制版的 Windows 多账号工具</p>
  <p>管理账号、启动游戏，按需添加识别统计和自动跟房。</p>
</div>

[下载安装](https://github.com/gjy991229/D2RHub/releases/latest) · [使用手册](docs/user-guide.html) · [更新记录](CHANGELOG.md) · [文档目录](docs/README.md)

当前版本 **v0.9.111**，配套加工器 **v1.4.0-beta.20**。本版增加加工前双向配套校验和小号跟房补发回车，支持房间工具 r32；详见 [本版说明](docs/releases/v0.9.111.md)。

## 做什么

D2RHub 在本机管理国服、国际服账号与各自的认证、游戏设置和启动状态。你可以单独启动账号，也可以保存常用账号组，一次启动并切换对应窗口。

多开始终可用，其他功能按需添加：

| 功能 | 用途 |
| --- | --- |
| Mod 管理 | 下载成品、加工已有 Mod、为账号选择 Mod |
| 识别与统计 | 通过游戏音频标记记录场景、掉落和刷图用时 |
| 自动跟房 | 主号创建房间，小号按规则跟随 |
| 桌面悬浮窗 | 在游戏外查看账号、邪恶区域和统计状态 |
| 桌宠 | 可关闭的 Bongo Cat 与装扮 |

**扩展功能与 Mod 分工不同：**扩展运行在 D2RHub 中，Mod 提供游戏内的能力。只多开不需要加工 Mod；识别需要声纹功能，自动跟房需要局内房间工具。它们可以放进同一个 Mod，再由需要的账号选用。

## 开始使用

1. 从 [Releases](https://github.com/gjy991229/D2RHub/releases/latest) 下载 Windows x64 安装器，安装并阅读首次使用说明。
2. 在“设置 → 运行环境”填写国服或国际服的游戏目录和存档目录。
3. 添加账号，按向导完成认证，然后从账号卡片启动游戏。重复添加其他账号即可多开。
4. 需要更多功能时，在“设置 → 扩展功能”添加对应扩展，再进入其设置完成准备。

[使用手册](docs/user-guide.html) 提供多开、识别、跟房和组合使用的分步路线。加工器 [d2r-audio-mod](https://github.com/gjy991229/d2r-audio-mod) 独立开源和发布；在“Mod 管理”中按需下载，不随 Hub 安装包内置。网络不便时也可以导入已下载的资源文件，仍会校验大小、摘要与兼容性。

加工前必须完成 Hub 与加工器互认。提示版本不匹配时，按页面要求更新 Hub 或加工器；互认失败会禁止加工，不再使用旧版继续处理。更新程序不会自动升级已有 Mod，房间工具 r32 需重新加工并重启游戏。只多开无需安装加工器。

## 本地数据与边界

账号、加密 Token、设置和统计存储在 `%APPDATA%\D2RHub`，日志位于程序旁的 `logs` 目录。Token 使用 Windows DPAPI 加密；程序不主动上传账号凭据，不读取或写入游戏内存，也不注入 DLL。

D2RHub 会使用进程、窗口、注册表、全局快捷键和按需启用的音频捕获能力。权限、联网范围、升级迁移及私密报告方式见 [安全政策](SECURITY.md)。公开反馈前请隐藏账号、Token、个人路径与日志中的隐私信息。

## 开发与贡献

需要 Windows、Node.js、Rust MSVC、Visual Studio C++ 构建工具和 WebView2。完整前置条件见 [开发指南](docs/DEVELOPMENT.md)。

```powershell
npm ci
npm run tauri dev
```

```powershell
python -m pip install -r scripts/publisher-requirements.txt
npm run check
npm run build
```

- [架构说明](docs/architecture.md)：产品边界、代码分层与配置归属。
- [贡献指南](CONTRIBUTING.md)：修改原则、检查命令和提交流程。
- [发布指南](docs/release-workflow.md)：准备安装器、加工器和 Mod，校验后发布与补传。

源码、运行资源和构建产物分别维护。安装包、生成的 Mod、个人配置和发布凭据不提交到 Git；公开 CI 只做验证，发布由维护者显式运行本地工具。

## English

D2RHub is a local Windows tool for managing multiple Diablo II: Resurrected accounts, separate CN/Global installations, authentication, launch groups, and per-account settings. Optional extensions provide audio-based run tracking, room automation, desktop overlays, and Bongo Cat.

Version **0.9.111** pairs with processor **1.4.0-beta.20**. Processing requires a successful two-way compatibility check. Follow the update message when either component is incompatible. This release adds bounded follower Enter repeats and room-tools r32; regenerate existing Mods and restart the game to use the new recipe.

[Download](https://github.com/gjy991229/D2RHub/releases/latest), configure a game and save directory, then add and initialize an account. The independently released [Mod processor](https://github.com/gjy991229/d2r-audio-mod) can be installed from Mod Management when needed. Account data stays on this computer; the app does not read/write game memory or inject DLLs. See the [manual](docs/user-guide.html), [development guide](docs/DEVELOPMENT.md), and [security policy](SECURITY.md).

## 许可

项目有权授权的源码与原创素材采用 [MIT License](LICENSE)，第三方素材与商标见 [第三方声明](THIRD_PARTY_NOTICES.md)。D2RHub 是非官方第三方项目，与 Blizzard Entertainment 无隶属、授权或背书关系。

[报告问题](https://github.com/gjy991229/D2RHub/issues) · [参与讨论](https://github.com/gjy991229/D2RHub/discussions)
