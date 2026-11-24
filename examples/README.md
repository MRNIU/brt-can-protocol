<!-- Copyright The brt-can-protocol Contributors -->
<!-- CMake 消费方接入和 Rust 可选帧适配示例。 -->

# 消费方接入与帧适配

Rust 调用方可运行 `cargo run --example encode_decode` 演示请求与应答的内存编解码。

## C17

将本仓固定到父工程的 `3rd/brt-can-protocol`。在 CMake 3.21 及以上版本中，对实际消费目标添加以下两行（本例目标名为 `sensor_app`）。

```cmake
add_subdirectory(3rd/brt-can-protocol)
target_link_libraries(sensor_app PRIVATE brt_can_protocol::brt_can_protocol)
```

在 C 源文件中包含 `<brt_can_protocol/protocol.h>`。父工程为库和消费目标选择一致的 CPU、指令集与浮点 ABI。宿主仿真可直接用宿主编译器构建同一份源码。

[最小 C 消费者](https://github.com/MRNIU/brt-can-protocol/blob/main/c/tests/consumer/main.c) 演示主机编码查询、传感器解码请求并编码应答，以及主机接收主动上报。调用解码器前，先从原生帧保留标准／扩展 ID 和实际 Classic／RTR／FD 形态，核对 DLC 和缓冲容量后只传有效字节。位置回传的有效长度为 7，传入整个八字节驱动缓冲会触发协议长度错误。

解码成功后，再按应答类型取出原始位置。静止时可以连续收到相同计数的合法帧，调用方可用各自的接收时刻刷新样本。主动上报和位置查询应答共用 `0x01`。传感器开启主动上报时，即使主机刚发过查询，下一帧的位置值也可能来自主动上报。

在本仓根目录可构建并运行这个父工程示例。

```sh
cmake -S c/tests/consumer -B target/c-consumer
cmake --build target/c-consumer
ctest --test-dir target/c-consumer --output-on-failure --no-tests=error
```

## embedded-can

依赖：`brt-can-protocol = { version = "0.1.1", features = ["embedded-can"] }`、`embedded-can = "0.4.1"`。支持 Rust 1.85。

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
brt-can-protocol = { version = "0.1.1", features = ["embassy-stm32"] }
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

芯片由消费方选择，一次只选一个具有 CAN 外设的芯片。Embassy 0.6 适配使用 Rust stable，CI 覆盖 `thumbv7em-none-eabihf` 上的 H723（FDCAN）和 F405（bxCAN）；F405 仅支持 Classic CAN。

```sh
cargo build --locked --lib --target thumbv7em-none-eabihf --features embassy-stm32,embassy-stm32/stm32h723vg
cargo build --locked --lib --target thumbv7em-none-eabihf --features embassy-stm32,embassy-stm32/stm32f405rg
```
