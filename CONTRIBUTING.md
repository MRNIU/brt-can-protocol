<!-- Copyright The brt-can-protocol Contributors -->
<!-- 贡献、验证与发布要求。 -->

# 贡献指南

协议问题请附供应商文档页码、CAN ID／帧形态、DLC 和有效字节。修改协议时同步更新 README 的支持范围与冲突说明，并使用独立给定的字节验证，避免重复用例。

保持默认 `no_std`、无 `alloc`、零运行依赖和 Rust 1.85；适配只转换帧，不添加设备流程。公开 API 使用中文 rustdoc，复用代码保留版权与许可归属。

验证命令集中维护在 [CI](https://github.com/MRNIU/brt-can-protocol/blob/main/.github/workflows/ci.yml)：默认与可选 feature 的测试、doctest、Clippy、rustdoc、MSRV 及 ARM 构建。Embassy 必须显式选择一个芯片。

发布前检查 `cargo package --locked` 和 `cargo publish --locked --dry-run`，确认包内无本地 path 依赖或临时文件。提交使用中文 Conventional Commit、`git commit --signoff` 和适用的 `Co-authored-by`。
