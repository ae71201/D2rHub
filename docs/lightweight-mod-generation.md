# 轻量 Mod 成品

LiteHub、BoHub、NullHub 直接制作、验证并发布成品。用户从“设置 → Mod 管理 → 下载与更新”安装，发布流程见 [资源分发方案](mod-resource-distribution.md)。独立加工器和 Hub 均不再提供这三个 Mod 的生成入口。

| 成品 | 兼容清单方案 | 默认启动参数 |
| --- | --- | --- |
| LiteHub | main | `-mod LiteHub -txt -assettestmode 1` |
| BoHub | filler | `-mod BoHub -txt -assettestmode 1` |
| NullHub | min | `-mod NullHub -txt -assettestmode 1` |

现有成品中的 `generation-manifest.json` 继续用于身份、数据版本和启动参数兼容校验；保留该格式不意味着仍支持现场生成。Mod 列表按清单识别方案，不根据名称猜测。成品可继续加工声纹、房间工具、双击 Esc 下一局和死亡退房功能。

发布时使用 `.\release.ps1 -Target mods` 打包已确认的三个成品目录；加工器独立构建、发布。修改成品不需要更新或维护生成配方。

验证：`npm test`、`cargo test --manifest-path src-tauri/Cargo.toml lightweight_mod::tests --lib`。

加工兼容性回归：设置 `D2RHUB_MOD_PROCESSOR` 为独立加工器 EXE，`D2RHUB_LIGHTWEIGHT_GAME_ROOT` 为游戏目录，`D2RHUB_MOD_PRODUCTS_ROOT` 为包含 LiteHub、BoHub、NullHub 成品的父目录，再运行 `cargo test --manifest-path src-tauri/Cargo.toml lightweight_all_processing_combinations --lib -- --ignored --nocapture`。测试直接读取现有成品作为来源，覆盖三个方案各 15 种功能组合及逐步增补、重复加工，不再调用生成命令。结果写入 `artifacts/lightweight-processing-<UUID>`，原成品保持不变。

矩阵通过后，将 `D2RHUB_PROCESSING_TEST_RESUME` 设置为结果目录，运行 `cargo test --manifest-path src-tauri/Cargo.toml lightweight_processed_same_name_replacement --lib -- --ignored --nocapture`，验证已加工成品的同名更新事务。测试不启动游戏或修改账号，游戏内效果仍需实机验证。
