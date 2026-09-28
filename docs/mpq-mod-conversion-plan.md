# MPQ Mod 识别与加工：实施规格

日期：2026-09-28。状态：加工器 CLI 与 Hub 独立解压入口已实现，尚未发布资源更新。

## 当前落地交互（优先于下文初始设计）

用户确认采用显式两步操作，不在“加工”中隐式解压：

1. Hub 扫描 `mods/<name>/<name>.mpq` 普通文件，加入原名预设，标记 MPQ 压缩包，操作显示“解压”。此时 source_eligible=false，不可直接加工。
2. 点击解压后，Hub 的 unpack_mod_capsule 命令调用加工器 capabilities 和 unpack-mpq CLI，展示进度与取消按钮；无需选择加工账号，也不改账号启动参数。
3. 成功后刷新同一个预设，显示备份路径及名称转义提示，操作切换为“加工功能”。用户再自行点击加工，沿用原有新产物流程。
4. 普通目录 Mod 仍显示原加工操作。中断事务保留原预设，点击解压由 CLI 恢复；未恢复前阻止该 Mod 启动/删除/加工。
5. Hub 不链接 StormLib、不自行移动原包/写转换日志；取消等待子进程退出后再调用 recover-mpq。

前后端能力字段：requires_unpack、unpack_recovery_required；事件 mod-mpq-progress；命令 unpack_mod_capsule。实现位于 src-tauri/src/mpq_mod.rs 和 Mod 库组件。

本地 debug 构建可调用同级 d2r-audio-mod 仓库的 Release 加工器以便联调；正式构建只使用原有校验过的加工器资源，发布时需要同步更新资源版本。能力不足明确提示更新，不尝试让旧加工器解压。

验证：新增界面测试覆盖独立解压、同预设刷新为加工、失败与取消；Hub 调用真实加工器转换 mini.mpq 副本的集成测试通过，原始下载包和 GUI 提取目录不变。

以下为初始实施规格；涉及自动开始加工的描述，以以上显式交互为准。

## 1. 目标与范围

支持 `mods/<name>/<name>.mpq` 中的普通 MPQ 文件作为预设 Mod。用户选择该源加工时，Hub 调用加工器 CLI 将其转换成同路径目录，原 MPQ 留在同目录的 back 中，然后按现有规则另建输出 Mod。

首版只自动处理标准安装布局。扫描不递归导入散落 MPQ，不自动修复外层名称不匹配，不自动剥离包内嵌套的 Mod 外壳，不改账号引用、源 Mod 名称或用户启动参数。

普通压缩 Mod 进入 create-with-source 流程；现有 processed Mod 的 augment/update 原位更新分支保持原规则。解压本身不授予任何“已加工/声纹就绪”资格。

## 2. 磁盘布局与不变量

转换前：`mods/Example/Example.mpq` 为文件。

转换后：

```text
mods/
  Example/
    Example.mpq/
      modinfo.json
      data/...
    back/
      <UTC时间戳>-<UUID>/
        Example.mpq
        conversion.json
        files.json
  Example-Enhanced/
    Example-Enhanced.mpq/
      modinfo.json
      data/...
```

事务期间在 Example 下增加 `.d2rhub-unpack-<UUID>/payload/` 和固定入口 `.d2rhub-unpack.json`。payload 内直接放包内资源；提交只把 payload 改名为 Example.mpq，不移动整个临时工作目录。

- backup 文件名保持 Example.mpq；唯一性由事务目录保证，禁止覆盖旧备份。
- 原压缩包校验值、大小记录在 conversion.json；files.json 保存规范相对路径、大小、SHA-256 和可用的条目标识。
- 原包内 modinfo.json 原样保留；首版要求其为合法 JSON，根目录存在 data，且至少含一个有效资源。不能要求必须有 Excel 表。
- 包内若只有 Example/Example.mpq/data 等多余外壳，返回 INVALID_MOD_LAYOUT，不能输出多套一层的目录。
- 不将 `(listfile)`、`(attributes)`、`(signature)` 等 MPQ 内部管理条目当作游戏资源；保留其他普通资源的路径与字节。
- 用户已有的外层说明文件不动。原加工流程只复制源 .mpq 目录，不复制 back、日志或临时目录。
- 首版备份随源 Mod 外层目录一起保留；现有删除 Mod 操作会删除该目录及其 back，删除说明必须明确这一点。

## 3. 加工器 CLI 合同

在 d2r-audio-mod 新增三个命令，不修改现有 augment 的输入语义：

```powershell
d2r-audio-mod.exe capabilities --json
d2r-audio-mod.exe unpack-mpq --source "D:\Games\D2R\mods\Example\Example.mpq" --events
d2r-audio-mod.exe recover-mpq --mod-directory "D:\Games\D2R\mods\Example" --events
```

capabilities 输出：

```json
{"schema_version":1,"capabilities":["mpq_unpack_v1","mpq_recover_v1"]}
```

unpack-mpq 不接受 --name、--output 或 --overwrite；目标就是原文件路径。--source 必须为绝对路径，父目录与 stem 按 Windows 名称规则匹配；保留磁盘实际拼写。已是目录时先处理未完成事务，再检查布局，返回 already_directory。

--events 下 stdout 只输出逐行 JSON；诊断进 stderr。退出码 0 才算成功；非零失败，有事件时以 code 分类。completed 必须唯一且在提交日志持久化后输出。recover-mpq 无事务时返回 no_transaction。

```json
{"type":"progress","operation":"unpack_mpq","phase":"extract","percent":35,"message":"正在解压源 Mod"}
{"type":"completed","operation":"unpack_mpq","report":{"schema_version":1,"status":"converted","mod_name":"Example","mod_directory":"D:\\Games\\D2R\\mods\\Example","mpq_directory":"D:\\Games\\D2R\\mods\\Example\\Example.mpq","backup_path":"D:\\Games\\D2R\\mods\\Example\\back\\<id>\\Example.mpq","transaction_id":"<id>","file_count":123,"uncompressed_bytes":456789,"archive_sha256":"<hex>"}}
{"type":"error","operation":"unpack_mpq","code":"UNRESOLVED_ENTRIES","message":"存在无法恢复原路径的资源，未替换原 MPQ","recovery_required":false}
```

建议共享类型：StorageKind(directory/mpq_archive)、UnpackReport、RecoveryReport、ProcessorCapabilities、UnpackErrorCode。事件协议 schema 独立于声纹协议及加工配方版本，不能为解压功能无故升级声纹协议。

稳定错误码至少包含：INVALID_MPQ、INVALID_MOD_LAYOUT、UNRESOLVED_ENTRIES、UNSUPPORTED_ARCHIVE、PATH_CONFLICT、UNSAFE_PATH、SOURCE_CHANGED、SOURCE_IN_USE、INSUFFICIENT_SPACE、EXTRACT_FAILED、COMMIT_FAILED、RECOVERY_REQUIRED、PROCESSOR_CAPABILITY_MISSING。

## 4. StormLib 集成与完整性边界

加工器新增本地 stormlib-sys crate，构建为 Windows x64 静态库；通过 CMake/build.rs 接入。安装用户不需要编译工具或额外 StormLib DLL。

2026-09-28 已通过官方远端标签和固定提交的文件核对源码基线：

- 官方仓库：https://github.com/ladislav-zezula/StormLib.git
- 选用标签：v9.40；固定提交：`6bb1882bd00ddbc3729cac5dac0fda81a61e5514`。`git ls-remote --tags` 返回该映射，头文件 STORMLIB_VERSION_STRING 为 9.40。
- 固定版本源码：https://github.com/ladislav-zezula/StormLib/tree/6bb1882bd00ddbc3729cac5dac0fda81a61e5514
- 主库许可证：MIT；分发保留原 LICENSE，并核对打包的内置压缩/加密依赖各自声明。
- 该提交 CMake 最低版本为 3.10；显式设置 `BUILD_SHARED_LIBS=OFF`、`STORM_UNICODE=ON`、`STORM_USE_BUNDLED_LIBRARIES=ON`、`STORM_BUILD_TESTS=OFF`。MSVC 运行库配置需与 Rust 目标一致；静态 StormLib 不等于自动静态链接全部 CRT。
- Unicode 构建下外部文件路径接口使用 UTF-16；归档内部名称仍遵循 StormLib 的 char 接口，FFI 不能把两者混用。
- 禁止追踪浮动 master。网页抓取的 master 元数据可能滞后，版本以实际 tag/commit 和头文件共同校验。

2026-09-28 技术验证进展：Windows x64 静态编译、Rust FFI 调用、中文路径、Zlib/Bzip2 加密条目往返已通过。现有 NullHub 目录只读重新打包再解压，21270 文件、78907956 字节逐字节一致。无 listfile 包确认出现 FileXXXXXXXX.xxx 占位名称。深层路径需要扩展长度格式，Rust 链接需显式 user32。可重复运行工具位于加工器 examples/stormlib-probe/，详见其中 README.md。

随后用户提供下载目录的原始 mini.mpq，已完成与 MPQEditor 手动提取成果对比：37811 文件、5272788 字节（包含 listfile），逐字节及独立 SHA-256 全部一致，原包摘要未变化。37660 个零字节文件必须保留。发现一个非 UTF-8 文件名，验证工具通过 GUI 名称反向还原原字节完成唯一映射；这仅验证对比，不代替正式转换的编码策略。完整条目覆盖的一般证明、更多原包回归、游戏实际加载、事务恢复和正式 CLI/Hub 接入仍待完成。

FFI 封装只暴露只读打开、条目枚举、文件读取及关闭；用 RAII 关闭所有句柄。相关官方入口包括 SFileOpenArchive、SFileFindFirstFile/NextFile、SFileOpenFileEx、SFileReadFile、SFileGetFileInfo。优先自行按块写文件，以便核对实际长度、计算摘要和限制输出总量。

必须通过测试证明枚举覆盖所有有效资源，不能用“枚举成功”代替“完整解压”。建立有效条目集合，以条目标识和 locale 区分；补充 listfile 只能增加可解析名称。遇到未知名称、伪造的 FileXXXXXXXX 名称且无法证明其是真实路径、无法读取、同路径多 locale 无法无损落盘时，停止转换。首版不做猜名、不取最后一个覆盖、不自动下载第三方 listfile；缺名字的包明确不支持。

拒绝依赖外部基包才能恢复的 patch archive。首版遇到目录无法表达的别名/多条目语义时拒绝，不承诺所有 MPQ 均可无损转换。需要真实 D2R 样本验证的名字枚举问题，是发布门槛，不是留到 Hub 接入后再处理的细节。

路径验证先于创建文件：拒绝绝对路径、盘符、UNC、..、ADS、设备名、尾随点/空格、大小写折叠冲突、文件/目录冲突；统一包内分隔符。父目录、back、事务目录拒绝符号链接和重解析点。枚举阶段统计总大小并检查空间，写入时继续核对实际字节数，空间不足不得进入提交。

## 5. 转换事务和恢复算法

CLI 在外层 Mod 目录持有 OS 文件锁，进程退出自动释放，unpack/recover 使用同一锁。Hub 的进程内 mutation lease 不能代替这个锁。原包读取期间禁止共享写入；提交重命名前关闭 StormLib 句柄，复核文件身份、大小和摘要，检测外部替换则失败。备份重命名后再次核对摘要，再发布目录。

固定日志字段：schema_version、transaction_id、mod_name、源大小/摘要、相对 stage/backup 路径、文件清单摘要、phase。路径由固定命名规则重新推导并检查，不能无条件信任日志中的任意路径。每次日志变更使用临时文件写入、flush/sync 和原子替换；文件内容同步完成后才记录 prepared。Windows 重命名和持久化行为需通过故障测试验证。

执行顺序：

1. 校验输入和布局；取得锁，恢复已有事务。
2. 写 extracting 日志，创建独占临时目录，提取资源。原文件仍在原位。
3. 校验资源覆盖、每文件长度/摘要和 Mod 布局；保存 files.json，写 prepared。
4. 写 backup_pending，关闭句柄，原文件以不覆盖方式重命名到唯一 back 目录。
5. 校验备份，写 publish_pending；payload 以不覆盖方式重命名为 Example.mpq。
6. 写 committed，保存最终 conversion.json，删除活动日志和空工作目录，输出 completed。

恢复不能只信 phase；两次重命名的间隙必须按实际路径及摘要判断：

| 原路径 | 匹配备份 | 恢复行为 |
| --- | --- | --- |
| 原始文件 | 无 | 原包保持原位；只清理已验证归属的本次临时目录，结束失败事务 |
| 不存在 | 有 | 将备份以不覆盖方式移回原路径，记录 rolled_back；下次转换重新解压 |
| 目录 | 有 | 若尚未 committed，按完整清单校验目录；匹配则补记提交，不匹配则阻止使用并保留全部材料 |
| 原始文件 | 有 | 若不是明确记录的合法已完成状态，视为冲突，保留双方并报错 |
| 其他组合/摘要不匹配 | 任意 | RECOVERY_REQUIRED；不猜测、不删除可恢复数据 |

持久化 committed 后只做收尾，不因用户后来编辑目录而强制恢复旧包。日志缺失时不递归寻找并删除未知临时目录。每次恢复重复执行必须幂等。

Hub 取消 CLI 后必须等待子进程完全退出，再运行一次 recover-mpq。此恢复不沿用已取消任务的取消信号；持有 mutation lease 直到恢复完成。整机或 Hub 崩溃则下次使用前恢复。后续 augment 失败不回滚已完成的源转换。

## 6. Hub 调用顺序与状态

新增 InstalledMod/ModCapsule 字段 storage_kind 和 requires_unpack。source_eligible 表示允许选择为候选源，不保证一定解压成功。扫描只做普通文件类型及标准布局识别；StormLib 深度校验在加工任务内执行，避免扫描每个压缩包。文件扩展名不证明内容有效，非法 MPQ 加工时明确报错。

当发现活动转换日志，即使原 .mpq 暂时不存在，也保留该预设身份并标记待恢复，不删除账号关联、不创建第二个预设。压缩包不跑依赖目录的 lightweight inspect，也不授予功能组能力；转换后重新执行已有凭证和源适用性校验，不能绕过旧版加工 Mod 限制。

prepare_audio_mod_impl 的顺序：

1. 取得现有 mod_mutations lease，解析游戏安装和账号。
2. 恢复该源未完成的 MPQ 转换，以及已有加工产物替换事务。
3. 校验输出名称、冲突、功能选择、源身份；拒绝输出与源同名（忽略大小写）。
4. 若为 MPQ 文件：检查加工器 capabilities，检查所有相关运行实例占用，调用 unpack-mpq。
5. 核对 completed 路径确实是预期目录，退出码为 0，磁盘布局和凭证通过现有校验。
6. 调用原 run_audio_mod_generator(augment)，输出新名称；沿用现有输出校验及账号绑定规则。
7. 成功或失败均刷新目录/源状态；若已经转换成功，失败提示同时显示备份路径。

进度区间建议：检查 0–5、源转换 5–30、加工 30–90、已有校验和收尾 90–100。目录源跳过转换阶段。适配器增加 progress_floor，使用 floor + percent * (ceiling-floor)/100，防止第二个 CLI 重新从 0 开始导致倒退。

每次启动相关游戏安装前，在现有启动互斥范围内检查活动 MPQ 日志并恢复；不能依赖 core_recovery_complete 的一次性缓存。扫描可以只标记待恢复，不偷偷变更文件。无法恢复时阻止相关 Mod 启动/删除/加工并给出具体路径，不阻止无关安装使用。旧加工器缺恢复能力时提示更新，不自行越过日志启动。

capabilities 结果按加工器实际资源身份（路径及已校验摘要）缓存。旧加工器可继续处理目录源；只有 MPQ 转换要求新能力。资源解析的旧版本回退不能导致反复选择不支持解压的可执行文件。

## 7. 文件级实施清单

### d2r-audio-mod

| 文件 | 操作 |
| --- | --- |
| Cargo.toml、Cargo.lock | 增加本地 StormLib 封装依赖并锁定构建依赖 |
| crates/stormlib-sys/（新增） | 官方源码版本、构建脚本、最小 FFI、许可证 |
| src/mpq/archive.rs（新增） | RAII 只读打开、完整枚举、流式读取 |
| src/mpq/layout.rs（新增） | 标准目录、包内路径、冲突和资源布局验证 |
| src/mpq/transaction.rs（新增） | 日志、备份、提交、恢复和 OS 文件锁 |
| src/mpq/mod.rs（新增） | unpack/recover 入口、报告、事件与错误码 |
| src/main.rs | 三个命令独立参数解析、帮助文本、JSON 事件输出 |
| src/generator.rs | 保留现有 find_source_layout 和复制加工语义；增加衔接回归测试 |
| README.md、LICENSE/第三方说明 | CLI、支持范围、备份位置和静态库许可 |

### D2rHub

| 文件 | 操作 |
| --- | --- |
| src-tauri/src/audio_mod.rs | InstalledMod 字段；prepare 转换前置步骤、失败恢复和重新校验 |
| src-tauri/src/audio_mod/validation/mod.rs | 文件/目录识别；待恢复状态；转换后原有凭证验证 |
| src-tauri/src/audio_mod/mpq.rs（新增） | unpack/recover 子进程适配、报告校验、待恢复日志发现 |
| src-tauri/src/audio_mod/generator.rs | 复用事件机制并增加进度起点 |
| src-tauri/src/mod_catalog.rs、domain/mod_catalog.rs | 存储类型与待恢复状态传到预设；保持名称身份；删除处理活动事务 |
| src-tauri/src/commands/launch.rs | 所有实际启动入口的每次恢复检查，不局限一次性初始化 |
| src-tauri/src/mod_resources.rs | 能力探测、缓存、MPQ 所需资源版本选择 |
| src/store 下对应 DTO、features/mods/workflow | 新字段、create-with-source 路由、进度和失败后刷新 |
| src/features/modCapsules 及 Mod 库展示 | MPQ 标记、待恢复提示、删除含备份说明 |
| resources/mod-resources-v2.json 及发布脚本 | 经测试的新加工器资源版本/摘要/大小；沿用现有分发流程 |

不动当前工作区其他文档整理变更。

## 8. 实施顺序和阶段退出条件

1. **StormLib 技术验证**：本地静态编译；人工创建可控包并读取；验证一个真实 D2R Mod 的条目完整性、中文安装路径和标准目录。无法证明完整枚举则先解决，不能继续发布转换功能。
2. **CLI 转换与恢复**：实现 unpack/recover/capabilities；故障注入覆盖每个日志写入和两次 rename 前后。退出条件为任意中断均保留可恢复原包，幂等恢复通过。
3. **Hub 后端接入**：扫描类型、源转换、能力检查、每次启动恢复、进度及报告校验。先跑真实 CLI 集成测试，再接 UI。
4. **前端接入**：普通 MPQ 可以选作新加工源，显示说明和备份路径；转换后身份不变，失败可直接重试。
5. **发布验证**：加工器 release 构建、无开发依赖干净机器验证、实际游戏加载转换前后对比、更新资源清单，然后发布 Hub 支持。

每阶段完成再进入下一阶段；用户本次要求为设计，以上是后续开发顺序，不表示已经执行。

## 9. 验收用例与执行命令

自动化测试使用 StormLib 创建的小型测试包，不依赖用户原始 Mod；真实游戏验证另列，不能以单元测试代替。

| 用例 | 必须断言 |
| --- | --- |
| 正常压缩包 | 转换后相对路径/字节与输入资源一致，备份 SHA-256 与原包一致 |
| 中文路径、空格、大小写扩展名 | 保留实际名称，CLI 参数不经 shell 拼接，正确落盘 |
| 嵌套外壳、损坏包、未知条目、多 locale | 转换失败，原包仍可用，不发布部分目录 |
| 路径穿越、大小写碰撞、链接 | 在写出越界数据或覆盖前失败 |
| 解压中磁盘满、原包被占用/替换 | 不发布新目录，错误明确 |
| 每个持久化/重命名边界 kill | 恢复后得到原文件或已完整验证的目录，备份不丢 |
| 连续两次转换/恢复、两个 CLI 并发 | 幂等或锁冲突，不覆盖备份、不重复提交 |
| 转换成功但 augment 失败 | 保留目录源和备份，预设身份不变，重试跳过解压 |
| 转换后完整加工 | 新 Mod 验证通过，目录源逐文件摘要不变，不复制 back |
| 无 Excel 的 Mod | 解压允许通过，具体加工能力由现有逻辑决定 |
| 解压后暴露旧加工凭证 | 重跑凭证校验，不能借压缩形式绕过限制 |
| 旧加工器 | 目录源正常，MPQ 明确提示更新，不启动转换 |
| Hub 取消与重启 | 先等待子进程退出再恢复；后续启动不会使用中间态 |
| 正在使用源 Mod | 转换前阻止，并指出占用实例 |
| 扫描 back 与临时目录 | 不产生额外预设，未完成事务不丢失原预设引用 |

建议检查命令（新增测试按此组织，当前尚未实现）：

```powershell
# D:\pro\d2r-audio-mod
cargo test mpq
cargo test
cargo build --release

# D:\pro\D2rHub
cargo test --manifest-path src-tauri/Cargo.toml mpq
cargo test --manifest-path src-tauri/Cargo.toml audio_mod
npm run check
npm run build
```

真实验收：同一标准 MPQ Mod 先运行记录表现，关闭全部相关实例，转换后以原名称和原参数运行；再加工到新名称并运行，检查所选功能、源文件摘要、备份摘要与账号预设引用。测试用例和记录全部通过后才能更新分发资源。

参考：[StormLib 官方源码与接口](https://github.com/ladislav-zezula/StormLib)。
