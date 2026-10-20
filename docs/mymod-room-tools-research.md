# MYMOD 房间工具研究（2026-10-19）

研究对象：`C:\Diablo II Resurrected\mods\MYMOD\MYMOD.mpq`。仅静态读取，未运行配置器、未改写 MYMOD、未向游戏发送按键。以下路径相对 `data/global/ui/layouts/`。

## 一键崔凡克及类似入口

- `CALL/OnekeyTravincalhd.json` 是 PausePanel，通过计时器打开 WaypointsPanelExpansion，0.1 秒后打开 CALL/WP-t-autuexitonekey，0.3 秒后卸载自身。
- `CALL/WP-t-autuexitonekeyhd.json` 是 WaypointsPanel。Tabs 设置 tabCount: 3、defaultTab: 2、onSwitchTabMessage: Waypoints:SelectWaypoint；末尾计时器卸载辅助面板、原传送面板和入口面板。
- 这是借用原生面板、默认选择事件和计时器串联动作；没有看到寻路或连续移动脚本。defaultTab 如何映射到当前游戏里的目标点、是否要求已解锁传送点及特定场景，必须实机验证，不能仅凭文件名断言可在任意地点传送。
- 多份 HUDPanel075hd.json / HUDPanel065hd.json 中的入口处于注释状态。文件存在不等于当前配置启用。CALL/WPautuexitonekeyhd.json 等有类似模板，需分别核对活动入口与配置器选择。

## 快速退出、再次开始、加入

- `CALL/helppanelexithd.json`：PausePanel 内的计时器发送 PausePanelMessage:ExitGame，随后卸载自身。
- `QuickHellGamePanelhd.json`：隐藏到屏幕外的 MainMenuHDPanel，计时器发送 CharacterSelect:LoadCharacter:2，然后卸载面板。名称表示快速地狱游戏，但这里没有自定义房名密码字段；不能当作按房名加入朋友房间的接口。
- `CALL/onekeyhd.json`、`CALL/HJWPautuexitonekeyhd.json` 同时请求退出面板和快速游戏面板。计时器值 -1 是该 Mod 的实际写法，尚未确认游戏调度顺序和加载竞态；不能当成已可靠串行的跨场景状态机。
- `CALL/mymodjoingamepanelhd.json` 和 `My1ModLobbyjoingamepanelhd.json` 使用原生 JoinGamePanel，保留房名、密码输入及 JoinGame:JoinGame。暂停菜单和背包入口通过 PanelManager 打开面板，局内快捷加入仍依赖原生表单上下文。

## 尸体拾取按钮为何能放到头像旁

`mymodsShowListLayouthd.json` 根类型仍是 PartyPanel。ListContainer / FriendsListTemplate 是玩家列表及每个玩家的行模板，行内保留名为 LootBodyButton 的 ToggleButtonWidget，位置变为 x:80, y:20，消息为 PartyPanelMessage:ToggleLootBody。同类结构还存在于 mymodsShowNamePassLayouthd.json、mymodsShowPassWordLayouthd.json 等文件。

关键是保留原生 PartyPanel、玩家行上下文、控件名称和切换消息，并修改外观位置，而不是新增一个能够任意指定玩家的权限 API。按钮资源的 untoggled/toggled 帧分别为 0/4，还有按下、悬停、禁用帧。

## 一键允许所有人 / 指定玩家的可行性

| 方案 | 当前证据 | 结论 |
|---|---|---|
| 在玩家行内点击按钮 | 原生按钮、列表绑定和切换消息存在 | 有静态依据；需专家角色实机确认开关方向 |
| 用 Timer 全局广播 ToggleLootBody | 未见目标参数；消息依赖行上下文 | 不能据此可靠实现 |
| 每一行无条件切换 | 切换不是“设置为允许” | 可能关闭已有授权，不应上线 |
| 快捷键打开列表，识别未授权行后点击 | 可基于可见姓名、状态和位置 | 可实验；需处理缩放、加入退出、重排、截断姓名与已授权状态 |
| 按名字授权某人 | 未见按名字或 ID 的消息接口 | 需定位可见姓名对应行，不能猜固定行号 |

推荐先做只读诊断原型，显示识别到的玩家姓名和授权状态，再验证快捷键。全员授权应跳过自己及已授权行，每次点击后重新确认姓名与状态；名单改变时停止并重读，无法识别时不点击。此轮未实现未经实机验证的自动授权，也未宣称游戏原生支持“允许所有人”消息。


## 设置项核验

官方资料确认辅助功能和部分设置存在，但未提供完整 Settings.json 键值表：

- [Blizzard：Technical Alpha Learnings](https://news.blizzard.com/en-gb/article/23695734/technical-alpha-learnings)
- [Blizzard：Accessibility](https://news.blizzard.com/en-gb/article/23700733/how-were-making-diablo-ii-resurrectedtm-more-accessible-to-everyone)
- [Blizzard：2.4 patch notes](https://news.blizzard.com/en-us/article/23788293/diablo-ii-resurrected-patch-2-4-ladder-now-live) 明确旧画面模拟有三种选项，当前值为 0 不能证明字段只有 0/1。

Chat Gem Mode、Private Game Security、Combat Feedback、Graphics Mode、Legacy Graphics Mode、AutoMapFade、HDR 参数、音频预设和历史缓存项均没有新增编辑入口。新增 13 个含义明确的开关只在原文件存在且为 0/1 或 true/false 时显示，不创造不存在的配置项，不改类型，不自动重写其他字段。

## 多屏证据与验证边界

原实现将 MONITORINFOEXW.szDevice（DISPLAY1/2）直接当持久 ID，前后端均以此映射保存布局。重新编号可能造成槽位关联到另一屏。新实现通过 EnumDisplayDevicesW 的 EDD_GET_DEVICE_INTERFACE_NAME 获取设备接口 ID；画面标签仍为 DISPLAYn，校准缓存使用同一身份。

旧方案没有设备身份，迁移按完整 bounds 匹配，再按名称，最后回退主屏；无法无损推断旧屏幕同时换位或更换接口的历史。新身份也不保证跨接口、驱动重装后保持不变。前后端回归测试覆盖名称交换、负坐标和枚举顺序变化。尚需真实双屏拔插、主屏切换、混合 DPI 与睡眠唤醒测试，静态风险点不等于用户现场已复现的唯一原因。

后续调整：软件端图像匹配和快捷键逐人反转已撤销。改为独立 `party_loot` Mod 加工模块，在左侧玩家列表逐人点击原生拾取按钮；不提供全员广播。
