<!-- Copyright The brt-can-protocol Contributors -->
<!-- 记录 0.1.0 发布准备的实际软件检查、包审计和证据边界。 -->

# 0.1.0 发布准备验证

验证日期：2026-09-15。目标是 `brt-can-protocol 0.1.0`，本记录只覆盖软件与发布准备。

## 环境与协议依据

- 主机验证目标：`aarch64-unknown-linux-gnu`。
- Rust stable：`rustc 1.98.1 (48a229cea 2026-09-01)`；MSRV：Rust `1.85.0`。
- ARM 目标：`thumbv7em-none-eabihf`、`thumbv6m-none-eabi`。
- 供应商说明书文件版本 V2.7，内部指令表标题 V2.1；正文、表格和冲突裁决见 [README](README.md)。
- 实现审查分别覆盖供应商协议、适配边界、发布元数据与最终差异。

## 依赖审计

已通过 `cargo tree --locked --edges normal` 和各 feature 的 `cargo metadata --locked --format-version 1` 核对：

- 默认运行依赖树只有本 crate，没有运行依赖。
- `embedded-can` 的运行依赖为 `embedded-can 0.4.1` → `nb 1.1.0`。
- `embassy-stm32` 使用注册表中的 `0.6.0`，自动启用 `embedded-can`，关闭 Embassy 默认 feature；芯片由检查命令或消费方显式选择。
- 库的依赖声明不绑定芯片；docs.rs 使用独立的 H723 文档构建配置。
- 所有外部依赖来自 crates.io，无本地 path 依赖、临时 patch 或其他工作区文件。
- Embassy 0.6 的元数据未声明 `rust-version`，本项目只验证其 stable 组合；Rust 1.85 承诺覆盖默认库和 `embedded-can`。

## 验证结果

以下检查均已通过。默认、`embedded-can`、Embassy 三种配置各自运行检查；没有使用缺少芯片选择的无效 feature 组合。可复现命令见 [贡献指南](CONTRIBUTING.md) 和 [CI](https://github.com/MRNIU/brt-can-protocol/blob/main/.github/workflows/ci.yml)。

| 检查 | 实际结果 |
|---|---|
| `cargo fmt --check` | 通过 |
| 默认 `cargo build --locked --lib` | 通过 |
| 默认测试、doctest、示例运行 | 17 项协议测试、1 项 doctest 通过；示例正常退出 |
| `embedded-can` 测试、doctest、示例运行 | 17 项协议测试、4 项适配测试、1 项 doctest 通过；示例正常退出 |
| Embassy + 显式 H723 测试、doctest、示例运行 | 17 项协议测试、9 项适配测试、1 项 doctest 通过；含 `Frame`／`FdFrame` 内存转换的示例正常退出 |
| 三种配置的 Clippy | `--all-targets -- -D warnings` 全部通过 |
| 三种配置的 rustdoc | `RUSTDOCFLAGS="-D warnings"`、`--no-deps` 全部通过 |
| 默认 ARM 库构建 | `thumbv7em-none-eabihf`、`thumbv6m-none-eabi` 均通过 |
| Embassy ARM 库构建 | 显式选择 `stm32h723vg`、`stm32f405rg`，分别在 `thumbv7em-none-eabihf` 上通过 |
| Rust 1.85 默认库与 `embedded-can` | 两种配置的 `check --lib`、`test --all-targets`、`test --doc` 均通过 |
| docs.rs 配置 | H723 feature + `thumbv7em-none-eabihf` 的 rustdoc 警告视为错误构建通过 |
| `cargo tree`／`cargo metadata` | 默认与两个可选 feature 的依赖来源、方向和芯片选择已核对 |
| `cargo package --locked --allow-dirty` | 包内源码独立解包编译通过 |
| `cargo publish --locked --dry-run --allow-dirty` | 通过；最终提示 `aborting upload due to dry run`，没有上传 |
| 独立消费方构建 | 从发布包解出的库，加上 `examples/README.md` 两个原样 Rust 片段，在消费方显式选择 H723、F405 后分别通过 ARM 构建 |

核心软件矩阵共 27 个命令，失败数为 0；其外另完成 docs.rs 配置、依赖、独立消费方和打包检查。17 项协议测试使用独立字节或协议长度表，包括所有功能的响应截断检查；没有忽略的测试。

开发复核中发现的 `FdFrame` 容器 Classic RTR 数据切片错误已修复：RTR 仅保留 DLC，数据分支才借用有效前缀；专项回归测试已通过。早期格式差异已修正，最终没有未解决的检查失败。

## 发布包审计

提交前为检查本轮新增文件使用 `--allow-dirty`；它只允许打包未提交文件，不跳过解包编译验证。CI 使用不带该参数的干净工作区命令。

- 包含 18 个文件：6 个源码文件、2 个测试文件、2 个示例文件、README、CONTRIBUTING、VALIDATION、LICENSE、Cargo.lock，以及 Cargo 生成／规范化的 Cargo.toml、Cargo.toml.orig、`.cargo_vcs_info.json`。
- AGENTS、CI 配置、本地构建产物、供应商 PDF 和临时消费方均不在发布包中。
- 所有 `include_str!` 输入位于包内，规范化 Cargo.toml 的依赖均来自注册表；没有工作区外文件、本地 path 依赖或临时 patch。
- 独立消费方仅在检查工程中临时指向解包目录；发布包自身没有该 path 声明。该消费方独立解析依赖，并验证了最新可解析依赖组合，而非依赖本库 Cargo.lock 的传递锁定。
- 原始 MIT 版权 `Niu Zhihong` 与许可条款保留，新增本项目贡献者归属，帧模型和适配中复用的实质代码保留 odrive-can-protocol 归属。

软件与包检查已达到可发布状态；这不等同于 crates.io 已存在可下载的 0.1.0。

## 未执行与边界

- 未执行真实 `cargo publish`、push、tag 或 GitHub Release。
- GitHub Actions 工作流尚未在远端触发；本地执行相应检查不能冒充远端 CI 运行。
- 未连接设备、修改设备参数或执行实板通信；未修改消费方源码。
- 说明书未给出独立主动回传 FUNC／布局，尤其无符号速度的返回格式。本库只实现有依据的字节布局，不把模式设置值当作返回 FUNC。
