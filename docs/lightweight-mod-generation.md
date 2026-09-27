# 轻量 Mod 与独立生成器

默认入口：设置 → Mod 管理 → 下载 Mod 与加工器。标准成品从 Releases 下载并安装到当前国服/国际服游戏目录。独立生成器继续支持定制生成，以下记录生成清单和兼容性验证。完整下载与更新规则见 [资源分发方案](mod-resource-distribution.md)。

| 方案 | 生成器配置 | 默认启动参数 |
| --- | --- | --- |
| LiteHub | main | `-mod LiteHub -txt -assettestmode 1` |
| BoHub | filler | `-mod BoHub -txt -assettestmode 1` |
| NullHub | min | `-mod NullHub -txt -assettestmode 1` |

Hub 为三个方案统一使用 `-txt -assettestmode 1`，同时兼容旧生成清单中缺少这些选项的原始启动参数。继续加工后的成品采用加工流程的完整启动参数。

本地加工需要独立加工器及本机原版资源。加工器从资源页按需安装，不打入 Hub 安装包。游戏资源版本由生成器校验，成品数据版本随实际读取的原版资源。

后台任务支持进度、取消及失败重试。生成和音频加工共享互斥锁。成品先写入 mods 下本次任务的临时目录，检查报告、目录身份、数据版本、文件数量和总字节数后再改名发布。同名目录不会覆盖。任务失败或取消只清理自身目录；无法确认子进程退出时保留目录供诊断。

列表根据生成清单识别轻量方案，不根据名称猜测。纯轻量成品按上表提供默认参数；实际继续加工后的成品沿用加工流程的参数。

验证：`npm test`、`cargo test --manifest-path src-tauri/Cargo.toml lightweight_mod::tests --lib`。真实原版资源冒烟测试需先设置 `D2RHUB_MOD_PROCESSOR` 为独立 EXE，再设置 `D2RHUB_LIGHTWEIGHT_GAME_ROOT`，再运行 `cargo test --manifest-path src-tauri/Cargo.toml bundled_generator_outputs_pass_hub_validation --lib -- --ignored --nocapture`，三个方案在独立临时目录生成并接受 Hub 校验，不启动游戏或修改账号。

加工兼容性回归：设置同一个游戏目录环境变量后，运行 `cargo test --manifest-path src-tauri/Cargo.toml lightweight_all_processing_combinations --lib -- --ignored --nocapture`。测试覆盖三个方案各 15 种功能组合及逐步增补、重复加工；四个单项和四项全选从纯轻量成品开始，其他包含声纹的组合从声纹成品增补，避免重复编码全部音频。每项调用由 `D2RHUB_MOD_PROCESSOR` 指定的独立加工器及 Hub 正式校验函数，同时检查版本和死亡退房开关。日志与结果写入 `artifacts/lightweight-processing-<UUID>`，保留基线及最终成品，便于资源对比。

矩阵通过后，将 `D2RHUB_PROCESSING_TEST_RESUME` 设置为该结果目录，运行 `cargo test --manifest-path src-tauri/Cargo.toml lightweight_processed_same_name_replacement --lib -- --ignored --nocapture`，验证已加工成品的同名更新事务。上述测试不操作用户安装目录中的 Mod，也不修改账号配置；游戏内效果仍需实机验证。
