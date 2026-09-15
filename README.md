<!-- Copyright The brt-can-protocol Contributors -->
<!-- 项目用法、协议依据与支持范围。 -->

# brt-can-protocol

布瑞特 BRT CAN 协议编解码库。Rust 2024，默认 `no_std`、无 `alloc`、零运行依赖，MSRV 为 Rust 1.85。库只转换协议值和 CAN 帧；收发、时序、重试、标定、物理换算与控制由调用方负责。

## 协议依据

协议依据：[《拉绳位移传感器 CAN 通信 V2.7》](https://www.buruiter.com/wp-content/uploads/2025/10/004-%E6%8B%89%E7%BB%B3%E4%BD%8D%E7%A7%BB%E4%BC%A0%E6%84%9F%E5%99%A8CAN%E9%80%9A%E4%BF%A1-V2.7.pdf) 印刷页 13–19。说明书中的长度、端序和波特率勘误见 [ERRATA.md](https://github.com/MRNIU/brt-can-protocol/blob/main/ERRATA.md)。

## 安装与使用

```toml
[dependencies]
brt-can-protocol = "0.1.0"
```

```rust
use brt_can_protocol::protocol::{Address, Request, Response, encode_request, decode_response};
use brt_can_protocol::{FrameId, FramePayload, FrameRef};

let address = Address::standard(1);
let query = encode_request(address, Request::ReadPosition).unwrap();
assert_eq!(query.data(), &[4, 1, 1, 0]);

let bytes = [7, 1, 1, 0x45, 0x23, 1, 0];
let frame = FrameRef {
    id: FrameId::Standard(1),
    payload: FramePayload::Data(&bytes),
};
assert_eq!(decode_response(address, frame).unwrap(), Some(Response::Position(74_565)));
```

`protocol` 提供 `encode_request`、`decode_request`、`encode_response`、`decode_response`。编码结果使用八字节固定缓冲，`FrameRef` 借用输入数据。运行内存示例：`cargo run --example encode_decode`。

## 功能覆盖

所有请求和应答均支持标准／扩展 Classic CAN 数据帧。各字段的端序如下。

| FUNC | 功能 | 请求 DATA／范围 | 应答 DATA |
|---|---|---|---|
| `01` | 读取位置 | 固定 `00` | 小端 `u32` |
| `02` | 设置标准地址 | `u8`，`1..=255` | `u8` 状态 |
| `22` | 设置扩展地址 | 大端 `u32`，`0..=0x1fff_ffff` | 大端 `u32` 状态；`LEN=07` |
| `03` | 设置波特率 | `00..=04`：500、1000、250、125、100 kbps | `u8` 状态 |
| `04` | 设置查询／回传模式 | 下述七种模式 | `u8` 状态 |
| `05` | 设置回传周期 | 小端 `u16`，`50..=65535` µs | `u8` 状态 |
| `06` | 当前位置设为零 | 固定 `00` | `u8` 状态 |
| `07` | 设置递增方向 | `00` 顺时针、`01` 逆时针 | `u8` 状态 |
| `0A` | 读取速度 | 固定 `00` | 小端 `i32` |
| `0B` | 设置速度采样时间 | 大端 `u16`，`0..=65535` ms | `u8` 状态 |
| `0C` | 当前位置设为中点 | 固定 `01` | `u8` 状态 |
| `0D` | 设置当前位置计数 | 大端 `u32` | `u8` 状态 |
| `0F` | 当前位置设为五圈值 | 固定 `01` | `u8` 状态 |

模式值：`00` 查询；标准帧位置／有符号速度／无符号速度为 `AA/02/07`，扩展帧对应 `18/12/17`。响应解码支持 `01/u32` 位置和 `0A/i32` 有符号速度，按 FUNC 和字段布局解析。无符号速度模式支持设置，其回传布局未实现。

### 帧与状态边界

- 报文为 `[LEN, DEVICE_ID, FUNC, DATA...]`。CAN DLC 是帧控制字段给出的实际数据字节数；`LEN` 是 `DATA[0]`，包含自身。除 `0x22` 请求的兼容例外外，二者必须相等，按功能为 4、5 或 7 字节，且不接受填充尾部。`0x22` 请求的 DLC 固定为 7，解码兼容 `LEN=04` 和 `LEN=07`，编码统一输出 `LEN=07`；其应答只能为 `LEN=07`／DLC 7。
- 当前标准地址可为 `0..=255`；新标准地址限 `1..=255`。扩展地址模式的 DEVICE_ID 取 CAN ID 低八位，例如 `0x18fff225` 对应 `0x25`。`Address::extended(can_id, device_id)` 显式保存两个字段，不自动派生或校验该关系。设址后的地址匹配和广播处理由调用方负责。
- 位置、速度和未知状态保留全部数据位。ACK 状态 `0` 表示成功，其他值为错误码；请求匹配和设备操作结果由调用方处理。
- 解码先校验 ID 格式并拒绝 RTR／FD，再过滤其他 ID。匹配 ID 的帧先校验长度及 DEVICE_ID；未知 FUNC 返回 `Ok(None)`，错误报文返回 `DecodeError`。

## 可选帧适配

| feature | 提供的转换 | 条件 |
|---|---|---|
| `embedded-can` | 通用 Classic CAN 帧 | Rust 1.85；调用方确认实际帧不是 FD |
| `embassy-stm32` | 原生 `Frame`／`FdFrame` | 自动启用 `embedded-can`；消费方选择芯片，使用 stable 验证 |

具体依赖、目标与接入片段见 [examples/README.md](https://github.com/MRNIU/brt-can-protocol/blob/main/examples/README.md)。原生适配保留 FDF，短 FD 也不会伪装成 Classic 帧。

## 贡献与许可

验证与提交要求见 [CONTRIBUTING.md](https://github.com/MRNIU/brt-can-protocol/blob/main/CONTRIBUTING.md)。采用 [MIT License](https://github.com/MRNIU/brt-can-protocol/blob/main/LICENSE)。
