# 开发预览

先运行 `npm run dev`，再访问以下 Vite 路径。预览使用演示数据，不读取真实账号，也不证明游戏内操作成功。所有预览都不在 `vite.config.ts` 的发行入口中。

| 页面 | 用途 |
| --- | --- |
| `/visual-audit.html` | 真实设置、Mod、扩展与窗口组件的审查夹具；当前保留根路径 |
| `/dev/previews/pet-settings-review.html` | 桌宠设置，`?lang=en` 英文、`?state=new` 新用户 |
| `/dev/previews/pet-visual-review.html` | 桌宠装扮三帧，`?slot=hands` 手部、`?new=1` 新品 |
| `/dev/previews/ui_design_demos.html` | 历史界面方向草稿，不代表现行界面 |
| `/dev/previews/ui_settings_center_demo.html` | 历史设置稿，不代表现行交互 |

真实组件使用 `/src/...` 的 Vite 绝对引用，移动 HTML 后不改变组件来源。不要将预览入口加入发行构建，也不要让演示数据进入生产代码。手册截图的来源与日期记录在 [图示说明](../docs/guide-images/README.md)。
