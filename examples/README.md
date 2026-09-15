<!-- Copyright The brt-can-protocol Contributors -->
<!-- 可选帧适配的消费方依赖与内存示例。 -->

# 帧适配示例

运行 `cargo run --example encode_decode` 演示内存中的请求与应答，不连接设备。

## embedded-can

依赖：`brt-can-protocol = { version = "0.1.0", features = ["embedded-can"] }`、`embedded-can = "0.4.1"`。支持 Rust 1.85。

```rust
use brt_can_protocol::{EncodedFrame, FrameRef};
use brt_can_protocol::compat::embedded_can::InvalidFrameLength;

fn convert<F: embedded_can::Frame>(encoded: &EncodedFrame) -> Option<F> {
    encoded.to_embedded_can()
}

fn borrow<F: embedded_can::Frame>(frame: &F) -> Result<FrameRef<'_>, InvalidFrameLength> {
    FrameRef::from_classic_embedded_can(frame)
}
```

`F::new` 必须构造 Classic 数据帧，`None` 表示驱动构造器拒绝。通用 trait 不携带 FDF；输入必须已确认是 Classic 数据帧或 RTR，DLC 不超过 8 且不超过缓冲长度。协议层拒绝 RTR。

## Embassy STM32

```toml
[dependencies]
brt-can-protocol = { version = "0.1.0", features = ["embassy-stm32"] }
embassy-stm32 = { version = "0.6", default-features = false, features = ["stm32h723vg"] }
```

```rust
use brt_can_protocol::{EncodedFrame, FrameRef};
use brt_can_protocol::compat::embassy::FromEmbassyError;
use embassy_stm32::can::frame::{FdFrame, Frame};

fn convert(encoded: &EncodedFrame) -> Result<(), FromEmbassyError> {
    let classic = Frame::from(encoded);
    let container = FdFrame::from(encoded);
    assert_eq!(FrameRef::try_from(&classic)?, FrameRef::try_from(&container)?);
    Ok(())
}
```

两种输出容器均为 Classic 数据帧。接收时原样保留 FDF，非法头部长度返回 `FromEmbassyError`；协议层拒绝 FD，包括短 FD。`FdFrame` 中的 Classic RTR 只保留 DLC。

芯片由消费方选择，一次只选一个具有 CAN 外设的芯片。使用 stable 验证 Embassy 0.6，不承诺其 MSRV 为 1.85。已验证 `thumbv7em-none-eabihf` 上的 H723（FDCAN）和 F405（bxCAN）；共享帧类型不代表 F405 支持 FD 收发。

```sh
cargo build --locked --lib --target thumbv7em-none-eabihf --features embassy-stm32,embassy-stm32/stm32h723vg
cargo build --locked --lib --target thumbv7em-none-eabihf --features embassy-stm32,embassy-stm32/stm32f405rg
```
