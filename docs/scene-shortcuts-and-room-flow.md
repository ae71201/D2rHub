# 快捷场景模块与局内房间流程对照

核对日期：2026-10-19。加工器源码来自相邻 `d2r-audio-mod`，MYMOD 来自本机游戏 mods/MYMOD。以下是静态代码核对，不代表游戏内行为已验证。

> 整理说明（2026-10-20）：下文 `dev/` 路径是历史验证输出位置，临时成品、提取副本和诊断快照已清理，不作为仓库交付物。实现与校验依据保留在源码、测试和 `resources/scene-shortcuts-*.json` 中。

## 当前使用条件

场景重开和左侧玩家尸体拾取按钮均使用游戏原生控件与消息链，无需图像识别或 OCR。场景重开需在 A3、崔凡克或 A4 小站处重开或创房。下文保留配方演进记录，当前使用条件以本节为准。

## 局内创建与加入

| 环节 | 当前加工器 | MYMOD |
|---|---|---|
| 房间入口 | 从原版重建两份高清暂停菜单，注入键盘导航入口；Hub 发送 Esc、左/右两次、Enter | 通过背包、暂停菜单或辅助面板打开原生/自定义房间面板；不同入口不完全相同 |
| 表单隔离 | 大厅表单与 D2RHubInGameCreateGame / D2RHubInGameJoinGame 分离 | 原生面板以及 CALL/mymodcreate、CALL/mymodjoin 等自定义面板并存 |
| 局内创建 | 按钮及输入框回车路由到 D2RHubCommitCreateGame；10 ms 打开暂停菜单，50 ms 按子节点顺序排队退出、CreateGame:CreateGame、清理 | CALL/mymodcreategamepanelhd.json 的创建按钮和输入框回车直接发送 CreateGame:CreateGame |
| 局内加入 | r33 按钮及输入框回车直接发送 JoinGame:JoinGame，不走退出控制器 | CALL/mymodjoingamepanelhd.json 的按钮和房名/密码回车直接发送 JoinGame:JoinGame；背包快速加入按钮打开 JoinGamePanel |
| 创建难度 | 加工表单追加 50 ms 的 CreateGame:SetDifficulty:2，默认地狱 | 表单提供 0/1/2 难度按钮；快速地狱是另一条 CharacterSelect:LoadCharacter:2 链路 |
| 房名密码 | Hub 提供规则、序号、预填、前台粘贴/后台按键、账号租约和失败处理 | JSON 主要提供原生缓存与输入框，未见与 Hub 等价的跨账号规则/序号系统 |
| 下一局地狱 | 独立可选双 Esc 模块；10 ms 打开暂停菜单，50 ms 排队退出和 CharacterSelect:LoadCharacter:2 | QuickHellGamePanel 本身仅发送 CharacterSelect:LoadCharacter:2；CALL/onekey 又会同时打开退出辅助面板和快速游戏面板 |
| 原 Mod 外观 | 两份高清暂停菜单从原版重建，会覆盖源 Mod 在这些文件上的自定义内容；其他 HUD 内容按加工规则保留 | 多种 HUD 比例、背包及配置器变体，启用情况取决于最终写入的文件 |

相同消息不等于相同时序。当前加入已与 MYMOD 的直接提交方式一致，不应因为 MYMOD 的“退出再下一局”文件而给加入流程重新加上退出。创建是否也能改成直接提交，需要用相同游戏版本和场景做对照核查；此次不擅自改动现有创建链路。

“创建表单直接提交创建”具体指：玩家仍先打开表单、填写房名密码，再点击创建按钮或在输入框按回车。MYMOD 将这个按钮/回车事件直接连接到 CreateGame:CreateGame；加工器的局内表单将同一事件连接到辅助控制器，先安排退出再提交创建。这里的“直接”描述按钮事件走哪条消息链，不是没有表单、没有按钮或自动创建。

证据位置：加工器 `src/generator.rs` 的 room_submission_layout、patch_room_form_layout、quick_recreate_layout、install_in_game_room_tools；Hub `src-tauri/src/capabilities/room_automation_windows.rs` 的 open_room_form；MYMOD `data/global/ui/layouts/CALL/mymodcreategamepanelhd.json`、`CALL/mymodjoingamepanelhd.json`、`CALL/onekeyhd.json`、`QuickHellGamePanelhd.json`、`controller/playerinventoryexpansionlayouthd.json`。

## 快捷场景加工模块的接入方案

独立可选功能组 `scene_shortcuts` r1 已接入加工器、Hub 单个/批量加工与成品校验，不依赖声纹、不强制安装房间工具，也不默认勾选。界面名称为“场景重开”。

1. 加工器：独立实现文件，CLI 功能开关，单独 recipe/fingerprint；覆盖 minimal、augment、已有声纹复用三条构建路径。
2. Hub：单独加工选项、批量加工、已有功能继承、输出文件校验与缺失模块错误；旧加工器不认识新开关时明确报错，不能假报成功。
3. 配方：记录目标、触发方式、原始布局来源、生成文件和时序；仅替换本模块命名的节点，重复加工幂等。
4. 共存：不覆盖现有创建/加入导航；提供顶部按钮和游戏帮助键入口。明确提示替换高清帮助页，原页保存到 `.mpq/d2rhub-scene-shortcuts-original-help.json`，不改用户按键绑定。
5. 目标：必须区分崔凡克、憎恨囚牢第二层传送点与第三层劳模本体。MYMOD 的 levels.txt 中第二层有 Waypoint=26，第三层为 255；不能把“劳模”当作存在原生第三层传送点。
6. 验收：普通/专家角色、城内/野外、已解锁/未解锁、高清布局、现有房间工具和双 Esc 共存。使用时需在 A3、崔凡克或 A4 小站处重开或创房，采用游戏原生消息链，无需图像识别。

**用户确认的行为**：人在崔凡克，点击或按键后直接 loading，下一房间落在崔凡克小站。模块复现 MYMOD 的小站选择与重开链，而不是“创建后再走到指定地点”，也不提供猜测的劳模第三层索引。用户确认的是 MYMOD 行为；新模块仍待游戏内验收。

## r1 实现与使用

- `d2r-audio-mod/src/generator/scene_shortcuts.rs` 独立实现配方，CLI 使用 `--features scene-shortcuts`；可与 rooms、esc-next-game、audio 等组合。
- 从原版 CASC 读取传送面板，使用 MYMOD 辅助面板的 tabCount=4、defaultTab=2 和 Waypoints:SelectWaypoint 控件事件；保留 100/200/300ms 链路及退出/地狱载入消息。它们是已观察到的 UI 事件参数，不是新猜测的地图 ID。
- 顶部“场景重开”按钮与游戏当前绑定的帮助键进入同一链路；仅高清界面。传统画面、手柄触发和不同场景需要另行验收。
- 该链路使用 CharacterSelect:LoadCharacter:2，不使用自动跟房的房名、密码或递增序号。不要把它当作多人指定房间规则的另一种触发方式。
- 加工器会保护已有帮助页备份，拒绝降级未来版场景模块；Hub 校验入口、时序、传送控件、退出、载入、HUD 和备份文件。
- 初次测试建议复制成品单独选用，在可接受退出当前房间时，从已解锁的崔凡克小站场景触发。验收记录应包括游戏版本、角色模式、起点、触发方式和实际落点。劳模等其他地点不能仅凭崔凡克结果宣称支持。

本机验证已完成：独立 scene-shortcuts 成品、从该成品增补 rooms + esc-next-game 的组合成品均可生成；帮助页备份逐字节保留。Hub 接受组合成品，并拒绝在临时副本中将重开时序篡改为 0ms 的布局。加工器单元测试、前端功能选项测试、生产构建及说明书资源检查通过。测试输出放在 `dev/scene-shortcuts-review/`，没有安装到游戏或启动游戏；游戏落点尚待实机验收。

## 为什么不能直接照搬“一键崔凡克”文件

OnekeyTravincal 打开传送面板，然后打开另一个 WaypointsPanel。后者把 TabBar 的 onSwitchTabMessage 改成 Waypoints:SelectWaypoint，defaultTab 为 2。它借用了控件事件参数，不是一个带地图 ID 的显式“传送崔凡克”命令。部分 HUD 入口被注释，其他帮助页变体又加入了退出重开链路，必须确定实际启用变体。

## 一个按键点击所有当前玩家

从输入自动化角度，一个快捷键可以依次点击多个玩家行。若目标只是“把每个授权按钮点一次”，可实现宏，但第二次会把已有授权反向关闭，也可能误点因玩家退出而移动的行。

若目标是“让所有人都可以拾取”，需要以下流程：

- 打开固定的 PartyPanel 布局，获取当前可见玩家和按钮状态；只处理最多七个其他玩家。
- 排除自己、禁用按钮和已经允许的玩家。
- 每次点击前确认该行仍对应同一玩家；点击后确认已切换为允许，再处理下一行。
- 名单变化、聊天/弹窗遮挡、游戏失焦或状态不明确时停止。
- 指定某个人时按可见姓名匹配，不能用固定第几行代表某个玩家。

MYMOD 的 LootBodyButton 位于玩家 ListItem 模板内，消息是 PartyPanelMessage:ToggleLootBody。现有文件没有展示“允许所有人”或“SetLootBody=true”的接口，也没有按角色名字作为参数的消息。不能假定在多个按钮上加 acceptsReturnKey 就会广播到全部玩家。

推荐路线是固定列表布局加按键触发的状态识别/逐行点击。纯 JSON 广播暂缺依据；坐标宏可以做，但不应包装成可靠的“全员允许”。


## 左侧玩家尸体拾取按钮模块

已取消软件端图像匹配与快捷键反转方案。独立 `party_loot` 模块使用原生 PartyPanel / FriendsListTemplate / LootBodyButton，消息为 `PartyPanelMessage:ToggleLootBody`。位置沿用 MYMOD 的列表 x=81、行 y=260/height=174、按钮 x=80/y=20。普通来源从原版 CASC 构造独立面板；已有 MYMOD 列表则只合并按钮并复用启动器，保留其他玩家信息。没有全员广播、自动点击或键盘绑定。

同名加工先在同一 mods 目录内的暂存目录生成与校验，再将原目录重命名为 `<源名>_backup_<唯一编号>`，安装新目录。备份内文件不变；原始备份可作为后续重做来源。已加工来源保留其原有来源链。事务日志支持替换中断恢复，运行中的 Mod 拒绝替换。单项和批量加工均可选择。

### 拾取按钮 r2：JCY 兼容修复

实测反馈的 `jcy-x` 来自 `jcy`。其 HUDWarnings 在 8ms 关闭，而 r1 新面板在 1s 才打开；JCY 的 JcyPartyExtra 已有玩家行，拾取按钮却在 (9999,9999)。r2 优先合并已有 JCY 列表，保留 204 行距与原始内容，补齐原生按钮材质和消息。按钮对齐 MYMOD 已知的首行全局坐标 (161,280)，文字右移留出空间，玩家行点击宽度扩到 300。启动定时器移到第一项立即执行，清理 r1 的重复启动器。宿主校验新增加载时机与 JCY 行内绑定检查，r1 可升级到 r2，未来未知版本仍拒绝覆盖。

当前 `jcy-x` 的直接修补仅替换 JcyPartyExtra，沿用其现有 6ms 启动入口，保持当前已安装 Hub 的 r1 清单兼容；原文件保存在该 Mod 的 `d2rhub-backups/party-loot-20261019-171924/`。加工器与内置打包文件升级为 r2。按钮实际显示与点击效果仍须重启游戏后由用户验收。


## 2026-10-19 r2 实测反馈修正

r1 被用户实测为：远离可用小站回第三幕城镇，能看到小站时回第四幕；因此不能视为“保留当前场景”已实现。MYMOD 的 ydata/helpset/helppanelhd-WPonekey.json 注释为“小站重建回A4”，其 WPautuexitonekey 使用 tabCount=4；崔凡克专用 CALL/WP-t-autuexitonekey 使用 tabCount=3。r2 改用后者的 tabCount=3/defaultTab=2，保留现有 100/200/300ms 重开链。该参数是控件参数，不能解释为地图编号；修正后仍需在实际可交互小站附近验证落点，不保证任意距离或劳模场景。

JCY 的 HUDWarnings 在 8ms 关闭，r1 工具栏安排在 10ms，导致按钮未加载。r2 将工具栏启动器放在第一项立即执行，保留 JCY 原有关闭行为。版本和指纹同步更新，允许准确识别的 r1 升级，拒绝未来版本降级。

已直接修补 jcy-x 的 HUD 启动器、场景传送面板和两份模块清单，备份位于 d2rhub-backups/scene-r2-20261019-200450。重启游戏生效；软件使用支持 r2 的新构建。未自动启动游戏或执行重开。


### 20:32 配置器切换“一键崔凡克”后的再次核对

实际改动是 HudPanelhd 等比例布局中的 OnekeyTravincal 按钮由注释变为启用。其提示为“从A4城镇不用点传送站直接到定义传送点”；调用 CALL/OnekeyTravincal → WP-t-autuexitonekey，没有退出或创建消息。当前 helppanelhd.json 和 helppanel.json 仍为 HelpPanel/伊森之书，HUDWarnings 自动打开 OnekeyTravincal 的行仍被注释。因此尚未获得用户所指 H 一键重建功能的完整启用状态，不能把这一配置当成完整参考链已验证。之前 r2 将传送参数改为 tabCount=3 也仍是待核实的局部修正，而非完整功能复现。读取快照保存在 dev/mymod-active-travincal-review，供下一次实际配置变化对照。


### 启用“帮助快捷键：重建回3C或超市”后的完整差分

已获得完整生效配置。此次变化有两处：helppanelhd 从伊森之书变为小站重建控制器；HUDWarnings 的 1 秒 OpenPanel:CALL/OnekeyTravincal 启动项由注释变为启用。核心是跨进房的两段链路：H → WPautuexitonekey（tabCount=4/defaultTab=2）→退出/下一局地狱；新房 HUDWarnings → OnekeyTravincal → WP-t-autuexitonekey（tabCount=3/defaultTab=2）。前段回 A4 与后段去目的地是不同用途，不能把前段的4简单改成3代替后段。先前 r2 正是缺了新房启动后的目的地链，修改前段参数不能算复现完成。此结论来自配置器应用前后的实际文件差分；尚未实机运行加工产物。快照：dev/mymod-active-rebuild-review。


## r3：按已启用 MYMOD 的完整两段链修正

前段 H/按钮：原生小站面板 → 100ms 选择 tabCount=4/defaultTab=2 → 200ms 退出并重建地狱 → 300ms 清理。后段每次进房：HUD 立即打开独立 Entry 面板，1 秒后执行 Arrival，后者立即打开小站面板、100ms 用 tabCount=3/defaultTab=2 选择崔凡克、300ms 清理。Entry 在1.4秒自行卸载，不受 JCY 在8ms关闭 HUDWarnings 影响。传送清理不再提前卸载前段重建控制器。

保持 MYMOD 的行为：每次进房都会尝试传送，并非只在按 H 重建后执行，也不是保留任意当前地图。只有崔凡克参考链；实际游戏落点仍待用户测试。安装时去除源 MYMOD 的同用途 OnekeyTravincal HUD 启动项，避免重复执行。r1/r2 精确指纹可升级至 r3，未来版本拒绝降级。Hub 校验两套传送参数、独立进房控制器及全部时序。此次不修改用户的 jcy-x，由用户自行加工。


## r4：仅保留帮助快捷键入口

删除顶部场景重开按钮及启动器，升级旧版时移除本模块的旧工具栏文件。独立进房控制器仍在启动后1秒启动传送链、1.4秒卸载自身；1.4秒是兼容实现的生命周期安排，不是MYMOD参数或传送成功确认，也不代表按H后固定等待1.4秒。r1/r2/r3可升级，实际落点仍需测试。


## 升级校验修正

旧场景模块作为升级来源时按清单中的 r1/r2/r3 配方校验原布局；新生成产物仍必须满足 r4 指纹与布局，不能用兼容校验接纳旧版为新成品。修复拾取按钮和场景模块同时要求占据 HUD 第一项的冲突：允许多个立即启动项组成前缀，仍拒绝延迟项之前无法到达、或 HUD 已被关闭后的启动项。已只读验证用户 jcy-x（场景 r2 + 拾取 r2）和 r1/r2/r3/r4 样本，并验证新生成的场景+拾取组合产物。修改位于 Hub 校验器，加工器保持 r4；仅替换加工器不能修复旧 Hub 的此次错误。


## r5：直接 HUD 触发，移除独立进房控制器

用户确认：当前启用的 MYMOD 在崔凡克或 A3 主城能看见小站的位置按 H，可以直接加载下一房间并到达崔凡克；r4 成品却停在 A4。静态文件不能确定失败发生在哪一步。r5 去掉自行添加的 SceneEntry Panel 和1.4秒卸载，恢复参考中的 HUDWarnings 1秒直接启动 Arrival。JCY 的8ms HUD关闭动作延至1秒，传送启动放在关闭前；其余 HUD 子项保留。这是明确的兼容调整，不声称已实机验证。

两套 waypoint 控件改用当前 MYMOD 的 WPautuexitonekey/WP-t-autuexitonekey 文件快照，随加工器编译内置；仅重命名面板/引用和清理计时器，不再从原版控件拼装。模板比较测试验证非计时器字段一致。旧r1–r4作为升级来源按原配方校验，新产物要求r5，并拒绝旧独立控制器、顶部按钮或1秒前关闭HUD。用户自行更新jcy-x，本次未修改游戏目录。


### r5 实机反馈仍停 A4：未解决

用户反馈 r5 仍停在 A4，实际安装清单确认是r5，直接传送定时器存在。静态检查发现此前未覆盖的差异：JCY在1ms打开另一个HUDWarningsPanel（JcyHudWarnings），且主HUD通过ClosePanel关闭；有效MYMOD的副本启动在1秒，主HUD通过UnloadPanel卸载。尚无游戏运行时证据证明计时器被取消或未重新初始化，不能将上述差异直接认定为已证实根因。已请求区分冷启动第一局和后续重开的行为，未继续变更游戏目录或发布新的延时猜测补丁。快照保存在dev/scene-r5-failure-review。


## r6：原始链路文件照搬，HUD合并单列

已取消自行生成的D2RHubScene控制器。内置当前用户已确认可用MYMOD的10个原始文件（帮助页高清/原版、原版小站布局、CALL目录辅助面板及QuickHellGamePanel），原字节写入成品，不重命名面板、控件或消息。SHA-256清单在Hub resources/scene-shortcuts-v6.json；新增校验按此清单检查原样文件。覆盖前的源布局保存在d2rhub-scene-original，已有帮助页备份保留。

HUD不能整体覆盖，否则会引用源Mod不存在的其他功能。因此保留源HUD内容，移除旧D2RHubScene启动器，末尾放入MYMOD原来的1秒UnloadPanel:HUDWarnings、1秒OpenPanel:CALL/OnekeyTravincal两个计时器（顺序及重复名称保留）。JCY的OpenPanel:JcyHudWarnings由1ms改为1秒，这是兼容合并，不是原文件字节照搬。每次进入游戏都会尝试传送，实际游戏行为尚待用户验证，不能从静态比较宣称修复成功。此次未改动jcy-x。

验证：r1–r5来源校验、r6新产物校验、JCY+拾取组合、文件篡改与HUD冲突拒绝、10个文件与本机有效MYMOD逐字节比较均通过。


### r6仍停A4：静态一致不等于运行一致

用户反馈r6仍失败。已确认安装r6且10个参考文件全部逐字节匹配当前MYMOD。只读提取原版CASC的objects.txt/levels.txt作对照：MYMOD小站对象关键字段与原版无差异，MYMOD与jcy-x的A3主城/崔凡克/A4城镇关卡行只见声纹SoundEnv差异。仍未定位游戏内失败环节。准备了Direct（H只传送、不重建）与HudOnly（隔离JCY其他HUD启动项）的诊断布局和当前文件备份，位置dev/scene-runtime-probe；未应用到游戏目录，未发布新的修复配方。下一步必须通过运行结果区分HUD触发与目的地链执行，不能继续以静态校验宣称修复。


## r7：补齐配置器强制联动的 actinfo 小站映射

重新检查MYMOD配置器的PyInstaller归档（仅解压数据、未执行配置器）找到实际实现文档字符串：_apply_onekey_travincal_state 联动按钮、HUD文字、actinfo目标与“四幕传送点修改”；一键崔凡克强制勾选该修改，先复制参考actinfo再修改第5行M列（act4/waypoint4）为Act 3 - Travincal。先前r1–r6只关注UI调用，未覆盖该依赖，是明确遗漏。

实际比较：MYMOD act3 的waypoint2/3为Act 4 - Town/Act 3 - Travincal，jcy-x对应仍是Act 3 - Jungle 1/Act 3 - Jungle 2；act4若干额外槽位也不同。jcy-x虽已有act4/waypoint4=Travincal，单这一格并不代表完整参考列表相同。r7按当前可用MYMOD同步act3/act4全部9个waypoint字段，保留其他幕、其他列及源文件换行风格；保留覆盖前actinfo备份。

原版CASC与MYMOD的小站objects关键字段一致；MYMOD与jcy-x的levels逐字段比较只见声纹SoundEnv差异，因此本次不覆盖objects或levels。10个UI原始文件与r6相同，HUD接入也不新增延时猜测。Hub r7校验增加actinfo映射检查，旧版来源仍按旧配方接受。规则篡改测试必须能拒绝恢复为旧第三幕映射的产物。静态修正补齐了真实配置依赖，最终游戏落点仍待实测。
