# Hub Mod 设置与继承

Mod 管理中展开成品的设置，可编辑游戏数据版本；LiteHub、BoHub 及其加工成品还可编辑第四幕传送页第 4–9 项。NullHub 不提供快捷传送。

数据版本写入 `data/global/dataversionbuild.txt`，同时更新成品中的对应版本记录、文件摘要与大小。它只是游戏数据版本标记，不会转换数据表，也不能证明已适配新的游戏补丁。

保存前需要关闭游戏。编辑具有修改冲突检查、Mod 操作互斥和中断恢复。修改后的成品仍属于本地自定义内容，资源更新不会直接覆盖这些改动。

## 加工成品

- 新加工、同名升级会记录经过验证的 Hub 来源，不依赖输出目录仍叫 LiteHub 或 BoHub。
- 旧 beta18 加工成品可通过加工凭证及同目录安装的原始 Hub 成品恢复来源；只匹配名称不能获得编辑权限。缺少可验证来源时不开放设置。
- 快捷传送只修改目标加工成品，保留其他幕与第四幕原有前三项，不改源 Mod。
- 声纹识别、局内房间工具、双击 Esc、死亡退房依旧按实际加工功能组和文件验证启用。Hub 来源不会自动赋予未加工的功能。
- 普通第三方 Mod、自定义启动预设及尚未解压的 MPQ 不提供这两个编辑入口。

## 验证

`hub_mod_settings` 单元测试覆盖基础身份、衍生继承、非法版本、旧页面写入、中断恢复及第三方目录拒绝。真实加工用例 `real_beta18_descendants_keep_settings_and_processing_credentials` 使用 `D2RHUB_SETTINGS_GAME` 和 `D2RHUB_SETTINGS_PROCESSOR`，仅在临时目录生成三个加工成品，验证版本编辑、快捷传送及加工凭证保留；不修改原游戏 Mod。

浏览器预览：`/visual-audit.html?surface=settings&settingsTab=mod-processing&hubSettings=1&batch=1`，数据全部为演示夹具。
