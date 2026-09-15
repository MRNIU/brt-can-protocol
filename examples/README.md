<!-- Copyright The brt-can-protocol Contributors -->
<!-- 本文件说明唯一示例和可选原生帧转换的消费方接入条件。 -->

# 示例与可选帧转换

[`encode_decode.rs`](encode_decode.rs) 使用说明书中独立给出的字节演示位置读取请求和响应解码；它不连接设备，也不发送 CAN 帧：

```sh
cargo run --example encode_decode
```

默认 crate 保持 `no_std`、无 `alloc`、零运行依赖。示例二进制使用 `std`，只用于展示结果。可选功能只在协议帧与驱动的内存帧类型之间转换，设备初始化、总线收发、芯片配置、任务和时序由消费方拥有。

## `embedded-can`

```toml
[dependencies]
brt-can-protocol = { version = "0.1.0", features = ["embedded-can"] }
embedded-can = "0.4"
```

`EncodedFrame` 可构造为满足 `embedded_can::Frame` 的 Classic 帧，原生帧可借用为 `FrameRef`。具体的关联函数和转换错误类型见 rustdoc。

```rust
use brt_can_protocol::{EncodedFrame, FrameRef};
use brt_can_protocol::compat::embedded_can::InvalidFrameLength;

fn to_native<F: embedded_can::Frame>(encoded: &EncodedFrame) -> Option<F> {
    encoded.to_embedded_can()
}

fn receive_classic<'a, F: embedded_can::Frame>(
    frame: &'a F,
) -> Result<FrameRef<'a>, InvalidFrameLength> {
    FrameRef::from_classic_embedded_can(frame)
}
```

通用 `embedded-can::Frame` trait 不携带 CAN FD 标志。发送转换要求 `F::new` 构造 Classic 数据帧；接收转换要求调用方只传入已确认的 Classic 数据帧或 RTR，并保留原生 DLC 对应的有效字节。BRT 协议解码器仍会拒绝 RTR；若驱动能报告 FD，则应保留该形态并使用能够表达 FD 的入口，不能因数据长度不超过 8 字节而降级为 Classic。

## Embassy STM32

```toml
[dependencies]
brt-can-protocol = { version = "0.1.0", features = ["embassy-stm32"] }
embassy-stm32 = { version = "0.6", default-features = false, features = ["stm32h723vg"] }
```

`embassy-stm32` feature 会启用 `embedded-can`，并提供 Embassy Classic／FD 帧转换。芯片 feature 由消费方在自己的 `embassy-stm32` 依赖中显式选择；上例只是 `stm32h723vg` 的一个配置示例，不代表该库绑定该芯片。CI 还检查 `stm32f405rg`，确认芯片选择没有被固化到默认库。

该集成在 Rust stable 上与 Embassy 0.6 验证；不承诺 Embassy 组合的 MSRV 为 Rust 1.85。嵌入式目标也由消费方选择：默认协议库构建 `thumbv7em-none-eabihf` 与 `thumbv6m-none-eabi`，而 Embassy 示例需使用其目标芯片可用的 target 和 feature 组合。

当 feature 已启用时，`EncodedFrame` 可转换为 Embassy `Frame` 或 `FdFrame` 容器；两种转换都保留 Classic 数据帧形态。接收转换使用 `?` 先传播适配错误，再把 `FrameRef` 交给协议解码器：

```rust
use brt_can_protocol::compat::embassy::FromEmbassyError;
use brt_can_protocol::{EncodedFrame, FrameRef};
use embassy_stm32::can::frame::{FdFrame, Frame};

fn as_frame_ref<'a>(frame: &'a Frame) -> Result<FrameRef<'a>, FromEmbassyError> {
    FrameRef::try_from(frame)
}

fn as_fd_frame_ref<'a>(frame: &'a FdFrame) -> Result<FrameRef<'a>, FromEmbassyError> {
    FrameRef::try_from(frame)
}

fn encode_for_embassy(encoded: &EncodedFrame) -> Result<(), FromEmbassyError> {
    let classic = Frame::from(encoded);
    let fd_container = FdFrame::from(encoded);
    let classic_view = as_frame_ref(&classic)?;
    let container_view = as_fd_frame_ref(&fd_container)?;
    assert_eq!(classic_view, container_view);
    Ok(())
}
```

原生入口根据 FDF 标志保留 `FramePayload::Fd`；其协议解码结果为 `UnsupportedCanFd`。头部长度超过缓冲、Classic 长度大于 8，或 FD 字节长度不在 `0..=8, 12, 16, 20, 24, 32, 48, 64` 中时，先返回 `FromEmbassyError::InvalidPayloadLength`。`FdFrame` 容器中未置 FDF 的 Classic RTR 仍保留 DLC，随后由协议层拒绝。

实际交叉构建组合：

```sh
cargo build --locked --lib --target thumbv7em-none-eabihf --features embassy-stm32,embassy-stm32/stm32h723vg
cargo build --locked --lib --target thumbv7em-none-eabihf --features embassy-stm32,embassy-stm32/stm32f405rg
```

应选择具有 CAN 外设的芯片，并且一次只选一个芯片。H723 使用 FDCAN，F405 使用 bxCAN；共享 `FdFrame` 内存类型的构建成功不表示 F405 具备 CAN FD 收发能力。

## 版本可用性

`brt-can-protocol = "0.1.0"` 是发布后的依赖写法。当前仓库尚未向 crates.io 发布；在发布前可直接在本仓库运行示例，不要把本文中的版本片段理解为已可下载的包。
