<!-- Copyright The brt-can-protocol Contributors -->
<!-- 贡献、验证与发布要求。 -->

# 贡献指南

欢迎通过 [Pull Request](https://github.com/MRNIU/brt-can-protocol/pulls) 提交修改。

## 提交修改

- 说明要解决的问题、修改后的行为和兼容性影响。
- 协议变更同步更新 Rust、C17、README 的支持范围和相关勘误。两端分别读取 `tests/fixtures/brt_v2_7.tsv`，对照独立给定的字段与字节；只在接收时兼容的帧列为仅解码用例。
- 适配依赖通过可选 feature 接入，芯片选择由调用方负责；接入示例放在 `examples/`。
- 文档、rustdoc 与 C 公开头文件的 Doxygen 使用中文，代码标识符使用英文。公开 API 说明前提、单位、错误和返回语义。
- 保留版权和许可声明。提交使用中文 Conventional Commit、`git commit --signoff` 和适用的 `Co-authored-by`。

Rust 库保持单个 Rust 2024 library crate、`no_std`、无 `alloc`、默认零运行依赖和 Rust 1.85。C17 静态库独立构建，使用调用方提供的存储。两者只转换值与帧，设备通信与控制流程由应用实现。

## 验证

在仓库根目录运行基本检查：

```sh
cargo fmt --check
cargo test --locked --all-targets
cargo test --locked --doc
cargo clippy --locked --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --locked --no-deps
cargo +1.85.0 check --locked --lib
```

C 协议测试与 API 文档在仓库根目录运行。

```sh
cmake -S . -B target/c-host -DBUILD_TESTING=ON
cmake --build target/c-host
ctest --test-dir target/c-host --output-on-failure --no-tests=error
doxygen Doxyfile
```

完整检查矩阵见 [CI](https://github.com/MRNIU/brt-can-protocol/blob/main/.github/workflows/ci.yml)，涵盖 Rust 可选 features、MSRV 和 ARM 构建，以及 C 的 GCC、Clang ASan/UBSan、外部消费者宿主运行与 Cortex-M7 硬浮点链接。Embassy 检查需选择一个芯片，具体命令见[接入示例](https://github.com/MRNIU/brt-can-protocol/blob/main/examples/README.md)。PR 中请列出执行的检查及结果。

## 发布前检查

```sh
cargo package --locked
cargo publish --locked --dry-run
```

核对包内文件、版本号、API 文档和仓库链接，确保没有本地 path 依赖或临时文件。
