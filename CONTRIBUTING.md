<!-- Copyright The brt-can-protocol Contributors -->
<!-- 本文件约定协议贡献范围、可复现软件验证和发布前检查。 -->

# 贡献指南

欢迎提交 Issue 和 Pull Request。请提供可复现的最小代码、crate 版本和 Rust 版本。协议问题还应包含说明书版本及页码、CAN ID 与格式、实际帧形态、DLC 和有效字节；不要把固定八字节接收缓冲区的未使用部分当作有效数据。

## 维护边界

- 保持一个 Rust stable、Edition 2024 的 library crate。默认构建必须是 `no_std`、无 `alloc`、零运行依赖，并继续支持 Rust 1.85。
- 协议层只转换协议值和 CAN 帧。CAN 外设访问、收发、任务、时序、重试、状态机、标定、物理换算、限幅和控制许可由调用方负责。
- 保留标准／扩展 CAN ID 的格式、实际 RTR／FD 形态、未知状态码和完整状态位。不得因数据小于八字节而把 FD 当作 Classic 帧。
- CAN ID 和报文内 `DEVICE_ID` 必须独立表达；不得假定扩展 ID 的低 8 位、广播语义或地址切换后的操作结果。
- 文档与 rustdoc 使用中文，Rust 标识符使用英文。公开 API 要说明输入前提、单位、错误和返回语义。
- 每个手写文件保留合法格式的 `Copyright The brt-can-protocol Contributors` 版权注释及职责说明。复用实质代码时保留来源版权和 MIT 许可条款。

## 协议变更

协议修改必须核对 README 中链接的供应商说明书及其 SHA-256，并以独立给定的报文字节测试请求、响应、端序、`LEN` 与有效 DLC。供应商正文与示例冲突时，先在 README 的「协议依据与冲突裁决」中补充可审计规则，再调整实现和测试。

请求与响应是无状态的独立编解码入口。地址变更、应答等待、重试、超时和设备动作判断都属于调用方；不要把这些行为放进 codec。说明书未定义主动回传的 `FUNC` 或布局时，不要推测出新解码规则。

对于 `0x06`、`0x0C` 和 `0x0F`，说明书正文只给出 `u8` 宽度，唯一示例分别为 `00`、`01`、`01`。保持现有无参数 API 和固定字节校验；不得把其他 `u8` 解释为新语义，也不得将 codec 的拒绝描述为设备已经实测的拒绝行为。

## 软件验证

在仓库根目录执行。`--locked` 确保使用锁定依赖，`RUSTDOCFLAGS` 将文档警告视为错误。

```sh
cargo fmt --check

cargo test --locked --all-targets
cargo test --locked --doc
cargo clippy --locked --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --locked --no-deps

cargo test --locked --all-targets --features embedded-can
cargo test --locked --doc --features embedded-can
cargo clippy --locked --all-targets --features embedded-can -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --locked --no-deps --features embedded-can

cargo test --locked --all-targets --features embassy-stm32,embassy-stm32/stm32h723vg
cargo test --locked --doc --features embassy-stm32,embassy-stm32/stm32h723vg
cargo clippy --locked --all-targets --features embassy-stm32,embassy-stm32/stm32h723vg -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --locked --no-deps --features embassy-stm32,embassy-stm32/stm32h723vg

cargo build --locked --lib --target thumbv7em-none-eabihf
cargo build --locked --lib --target thumbv6m-none-eabi
cargo check --locked --lib --target thumbv7em-none-eabihf --features embassy-stm32,embassy-stm32/stm32h723vg
cargo check --locked --lib --target thumbv7em-none-eabihf --features embassy-stm32,embassy-stm32/stm32f405rg

cargo +1.85.0 check --locked --lib
cargo +1.85.0 test --locked --all-targets
cargo +1.85.0 test --locked --doc
cargo +1.85.0 check --locked --lib --features embedded-can
cargo +1.85.0 test --locked --all-targets --features embedded-can
cargo +1.85.0 test --locked --doc --features embedded-can

cargo tree --locked --edges normal
cargo metadata --locked --format-version 1 --no-deps
cargo package --locked
cargo publish --locked --dry-run
```

`embassy-stm32` 0.6 只在 Rust stable 组合中验证；本项目不把 Embassy 的 MSRV 声明为 Rust 1.85。为 Embassy 构建时必须显式选择一个芯片 feature，不能以没有芯片选择的 `--all-features` 代替。默认的 `cargo tree --edges normal` 应显示零运行依赖；`embedded-can` 及 Embassy 依赖只允许在相应 feature 下出现。

验证记录应如实区分已通过的软件检查、失败、未执行项和实板证据；发布前更新 [VALIDATION.md](VALIDATION.md)。软件检查、交叉编译、`cargo package` 和 `cargo publish --dry-run` 均不证明设备通信、写入参数或运动结果。

## PR 与发布

PR 请说明触发条件、变更后的行为、公共 API／兼容性影响、协议依据和实际验证。提交采用中文 Conventional Commit，并以 `git commit --signoff` 添加 DCO；AI 协作保留适用的 `Co-authored-by`。

发布前完成上述检查，审阅 `cargo package` 的内容并更新 [VALIDATION.md](VALIDATION.md)。`cargo publish --dry-run` 仅验证发布流程，不能创建真实版本。发布、推送、tag 与 GitHub Release 均需要维护者的明确授权。
