<!-- Copyright The brt-can-protocol Contributors -->
<!-- 本文件说明 BRT CAN 编解码范围、供应商依据、兼容性与使用方法。 -->

# brt-can-protocol

[![CI](https://github.com/MRNIU/brt-can-protocol/actions/workflows/ci.yml/badge.svg)](https://github.com/MRNIU/brt-can-protocol/actions/workflows/ci.yml)
[![MIT](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/MRNIU/brt-can-protocol/blob/main/LICENSE)
[![MSRV](https://img.shields.io/badge/MSRV-1.85-blue.svg)](https://blog.rust-lang.org/2025/02/20/Rust-1.85.0/)

`brt-can-protocol` 是布瑞特拉绳位移传感器 CAN 应用层协议的硬件无关编解码库。默认构建为 Rust 2024、`#![no_std]`、无 `alloc`、零运行依赖，MSRV 为 Rust 1.85。它只在协议值和 CAN 帧之间转换；不访问 CAN 外设，不发送或接收报文，也不管理超时、重试、任务、标定、物理单位换算、限幅或控制许可。

当前仓库处于发布准备阶段，尚未实际发布到 crates.io。发布完成后可添加：

```toml
[dependencies]
brt-can-protocol = "0.1.0"
```

Rust 中的 crate 名为 `brt_can_protocol`。

## 快速开始

请求与响应使用不同的函数。编码结果是固定容量的 `EncodedFrame`，`data()` 只暴露有效 DLC 前缀；解码输入则显式携带 CAN ID、帧形态和有效字节。

```rust
use brt_can_protocol::v2_7::{
    decode_response, encode_request, Address, Request, Response,
};
use brt_can_protocol::{FrameId, FramePayload, FrameRef};

let address = Address::standard(1);
let query = encode_request(address, Request::ReadPosition).unwrap();
assert_eq!(query.id(), FrameId::Standard(1));
assert_eq!(query.data(), &[0x04, 0x01, 0x01, 0x00]);

let bytes = [0x07, 0x01, 0x01, 0x45, 0x23, 0x01, 0x00];
let received = FrameRef {
    id: FrameId::Standard(1),
    payload: FramePayload::Data(&bytes),
};
assert_eq!(
    decode_response(address, received).unwrap(),
    Some(Response::Position(74_565)),
);
```

可运行的完整示例见 [`examples/encode_decode.rs`](https://github.com/MRNIU/brt-can-protocol/blob/main/examples/encode_decode.rs)：

```sh
cargo run --example encode_decode
```

## 支持矩阵

`v2_7` 是供应商 PDF 的说明书版本标识，并非已经核实的设备固件版本。该 PDF 的指令表标题为「V2.1」；本 crate 以其第 13–18 页的应用层布局与指令表为依据，因此模块命名采用说明书文件版本 `v2_7`。

| 范围 | API／格式 | 状态与限制 |
|---|---|---|
| 说明书协议 | `brt_can_protocol::v2_7` | 已实现该说明书明确给出的请求、响应与字段校验 |
| CAN 标识符 | 11 位 `Standard`、29 位 `Extended` | 支持；CAN ID 与报文内 `DEVICE_ID` 独立表达，不能从低 8 位推定其关系或广播语义 |
| CAN 帧 | Classic 数据帧，DLC 4、5 或 7（按功能） | 支持；`LEN` 必须等于实际有效字节数，尾部填充会被拒绝 |
| RTR 与 CAN FD | `FramePayload::Remote`、`FramePayload::Fd` | 协议层拒绝；FD 即使有效数据不超过 8 字节也不能交给 Classic 入口 |
| 读取位置／角速度 | `0x01` / `u32`、`0x0A` / `i32` | 支持，小端；数值仍是设备编码值，库不换算成长度或速度单位 |
| 设置命令回执 | `0x02`、`0x03`、`0x04`、`0x05`、`0x06`、`0x07`、`0x0B`、`0x0C`、`0x0D`、`0x0F` | 支持；`u8` 状态原样保留，不把未知状态码变成解码错误 |
| 扩展地址设置回执 | `0x22` / `u32` | 支持，小端并完整保留 32 位状态；调用方显式用新 `Address` 匹配后续应答 |
| 自动回传设置 | `0x04` 的七个模式值 | 设置请求和 ACK 支持；说明书没有给出主动回传独立的 `FUNC`、`LEN` 或布局，库不臆造映射 |

所有多字节字段使用 little-endian。`decode_response` 能解码地址匹配且布局符合已定义 `0x01`/`u32` 或 `0x0A`/`i32` 的帧，但它不根据某个待处理请求推断帧来源，也不声称该帧必然是查询响应而非主动回传。无符号角速度自动回传模式可以编码设置；其回传功能码和布局未在说明书中定义，不能据此猜测或解码。

### 指令覆盖

| FUNC | `Request` | 请求数据 | `Response` | 约束／单位 |
|---:|---|---|---|---|
| `0x01` | `ReadPosition` | 固定 `0x00` | `Position(u32)` | 设备位置编码值 |
| `0x02` | `SetStandardAddress(u8)` | 新地址 | `Ack` | 当前标准地址可为 `0..=255`；新标准地址仅 `1..=255`，成功后设备用新地址应答 |
| `0x22` | `SetExtendedAddress(u32)` | 29 位 CAN ID | `SetExtendedAddress(Status32)` | CAN ID `0..=0x1FFF_FFFF`；报文内 `DEVICE_ID` 不从该 ID 推导 |
| `0x03` | `SetBaudRate(BaudRate)` | `0..=4` | `Ack` | 500K、1M、250K、125K、100K |
| `0x04` | `SetReportingMode(ReportingMode)` | `00`、`AA`、`02`、`07`、`18`、`12`、`17` | `Ack` | 查询、标准／扩展位置或有／无符号角速度模式 |
| `0x05` | `SetReportingPeriod(u16)` | 小端 `u16` | `Ack` | 自动回传周期，`50..=65535` µs |
| `0x06` | `SetZero` | 固定 `0x00` | `Ack` | 唯一示例给出 `0x00`，其他 `u8` 未定义 |
| `0x07` | `SetDirection(Direction)` | `00` 或 `01` | `Ack` | 顺时针／逆时针 |
| `0x0A` | `ReadSpeed` | 固定 `0x00` | `Speed(i32)` | 设备角速度编码值 |
| `0x0B` | `SetSpeedSampleTime(u16)` | 小端 `u16` | `Ack` | 角速度采样时间，`0..=65535` ms |
| `0x0C` | `SetMidpoint` | 固定 `0x01` | `Ack` | 唯一示例给出 `0x01`，其他 `u8` 未定义 |
| `0x0D` | `SetPosition(u32)` | 小端 `u32` | `Ack` | 设备位置编码值 |
| `0x0F` | `SetFiveTurns` | 固定 `0x01` | `Ack` | 唯一示例给出 `0x01`，其他 `u8` 未定义 |

`Ack` 的具体命令使用 `AckCommand` 保留，状态使用 `Status(u8)` 保留。说明书仅定义状态值 `0` 为成功、其他值为错误码；该值不用于推导它对应的待处理请求、设备写入是否实际应用或掉电后是否保持。调用方负责地址变化后的收发顺序、匹配和动作是否真正完成。

## 报文边界与错误语义

应用层有效载荷固定为 `[LEN, DEVICE_ID, FUNC, DATA...]`。`LEN` 包含自身，最大为 8，且必须与 Classic CAN 的实际有效 DLC 相等。库使用 `FrameRef` 接收借用数据，因此不会把未使用缓冲区当作报文字节；`EncodedFrame` 使用八字节固定缓冲区，不分配内存。

`decode_request` 与 `decode_response` 仅解析各自方向已定义的布局。RTR 与 CAN FD 在 ID 匹配前即返回 `DecodeError`，即使它们属于其他设备。对于 Classic 数据帧，合法但 ID 不匹配的帧返回 `Ok(None)`；ID 匹配后，解码器依次检查数据上限、最小头部、`LEN` 与有效 DLC 相等、`DEVICE_ID`，然后才对未知 `FUNC` 返回 `Ok(None)`。匹配 ID 的长度、固定参数或范围错误返回 `DecodeError`。`encode_request` 与 `encode_response` 在编码前验证可表示的地址和参数范围，失败时返回 `EncodeError`。

标准／扩展格式是 ID 的一部分。`Address::standard(device_id)` 和 `Address::extended(can_id, device_id)` 要由调用方以实际协议配置显式构造；扩展 ID 不等同于 `DEVICE_ID`，库不保存地址变更前后的状态。

## 可选帧转换

默认库没有运行依赖。可选功能仅将内存中的协议帧转换为驱动类型，不会打开设备、读写总线或选择外设。接入方式和芯片选择见 [examples/README.md](https://github.com/MRNIU/brt-can-protocol/blob/main/examples/README.md)。

| feature | 提供的转换 | 构建前提 |
|---|---|---|
| `embedded-can` | 通用 Classic `embedded_can::Frame` 的借用／构造转换 | Rust 1.85；调用方确认输入不是 FD，通用 trait 不携带 FD 标志 |
| `embassy-stm32` | Embassy STM32 的 Classic／FD 帧转换 | 同时启用一个 `embassy-stm32` 芯片 feature；本 crate 只在 Rust stable 上验证 Embassy 0.6，不承诺其 MSRV 为 1.85 |

## 协议依据与冲突裁决

协议依据是供应商 PDF：[《004-拉绳位移传感器CAN通信-V2.7》](https://www.buruiter.com/wp-content/uploads/2025/10/004-%E6%8B%89%E7%BB%B3%E4%BD%8D%E7%A7%BB%E4%BC%A0%E6%84%9F%E5%99%A8CAN%E9%80%9A%E4%BF%A1-V2.7.pdf)，SHA-256 为 `d0f89ba75d4d77d09a1a96a15376b2b649ada959e83fc2d47f73433a9532cf1e`。仓库不分发该 PDF。

说明书内容存在互相矛盾的示例或提示，以下规则是本实现和测试的唯一裁决：

| 冲突 | 采用的规则 |
|---|---|
| 红框称仅标准帧，正文随后称支持标准帧和扩展帧 | 采用正文：标准和扩展 CAN ID 均支持 |
| `0x22` 示例写 `LEN=0x04` 且字节顺序不符 | 采用字段宽度和 `LEN` 定义：`LEN=0x07`，`u32` little-endian |
| `0x0B` 示例为大端 `03 e8` | 采用通用端序定义：1000 ms 编码为 `e8 03` |
| `0x0D` 示例的多字节顺序冲突 | 采用通用端序定义：`0x0001_2345` 编码为 `45 23 01 00` |
| LED 表写 150K，`0x03` 命令表写 125K | 采用可编码命令表：值 `3` 为 125K |
| `0x06`、`0x0C`、`0x0F` 正文仅称请求数据为 `u8`，唯一示例分别给出 `00`、`01`、`01` | API 只编码并接受这三个示例字节；其他值没有定义而被拒绝。这是 codec 的保守边界，不是对设备实测拒绝行为的声明 |

## 贡献与发布准备

软件检查、可选 feature、交叉编译和打包步骤见 [CONTRIBUTING.md](https://github.com/MRNIU/brt-can-protocol/blob/main/CONTRIBUTING.md)。实际执行结果、未执行项和 package 内容审计见 [VALIDATION.md](https://github.com/MRNIU/brt-can-protocol/blob/main/VALIDATION.md)；软件检查不等同于实板通信、地址写入或设备动作证据。

## License

本项目采用 [MIT License](https://github.com/MRNIU/brt-can-protocol/blob/main/LICENSE)，保留原有版权信息和许可条款。
