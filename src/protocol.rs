// Copyright The brt-can-protocol Contributors
// 本文件实现供应商 CAN 指令表的无状态、驱动无关编解码。

//! 布瑞特 CAN 应用层协议。
//!
//! 本模块提供地址、功能码和数据字段的无状态编解码，协议依据见 README。

use core::fmt;

use crate::{EncodedFrame, FrameId, FramePayload, FrameRef};

const MAX_STANDARD_ID: u16 = 0x07ff;
const MAX_EXTENDED_ID: u32 = 0x1fff_ffff;
const READ_POSITION: u8 = 0x01;
const SET_STANDARD_ADDRESS: u8 = 0x02;
const SET_EXTENDED_ADDRESS: u8 = 0x22;
const SET_BAUD_RATE: u8 = 0x03;
const SET_REPORTING_MODE: u8 = 0x04;
const SET_REPORTING_PERIOD: u8 = 0x05;
const SET_ZERO: u8 = 0x06;
const SET_DIRECTION: u8 = 0x07;
const READ_SPEED: u8 = 0x0a;
const SET_SPEED_SAMPLE_TIME: u8 = 0x0b;
const SET_MIDPOINT: u8 = 0x0c;
const SET_POSITION: u8 = 0x0d;
const SET_FIVE_TURNS: u8 = 0x0f;

/// 经检查的协议目标地址。
///
/// 标准地址的节点号与标准 CAN ID 都是同一个字节；扩展地址同时保留 29 位 CAN ID 和
/// 报文中的设备 ID。`0` 是可表示的当前地址，只有设定新的标准地址时协议禁止 `0`。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Address {
    id: FrameId,
    device_id: u8,
}

impl Address {
    /// 创建标准 CAN 地址。
    ///
    /// `device_id` 保持原始八位值；因此可表示说明书定义的当前地址 `0`。
    pub const fn standard(device_id: u8) -> Self {
        Self {
            id: FrameId::Standard(device_id as u16),
            device_id,
        }
    }

    /// 创建扩展 CAN 地址。
    ///
    /// `can_id` 必须在 `0..=0x1fff_ffff`；`device_id` 是数据域内独立的一字节设备 ID。
    /// 扩展地址模式下该字节取 CAN ID 的低八位，例如
    /// `Address::extended(0x18ff_f225, 0x25)`。
    /// 本函数保留调用方的显式值，不自动派生或校验二者关系；越界返回
    /// [`EncodeError::InvalidExtendedCanId`]。
    pub const fn extended(can_id: u32, device_id: u8) -> Result<Self, EncodeError> {
        if can_id <= MAX_EXTENDED_ID {
            Ok(Self {
                id: FrameId::Extended(can_id),
                device_id,
            })
        } else {
            Err(EncodeError::InvalidExtendedCanId(can_id))
        }
    }

    /// 返回 CAN 标识符及其标准／扩展格式。
    pub const fn id(self) -> FrameId {
        self.id
    }

    /// 返回数据域中的协议设备 ID。
    pub const fn device_id(self) -> u8 {
        self.device_id
    }
}

/// 说明书定义的五种 CAN 通信波特率。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BaudRate {
    /// `500 kbit/s`，设备默认值，线上值为 `0x00`。
    Kbps500,
    /// `1 Mbit/s`，线上值为 `0x01`。
    Mbps1,
    /// `250 kbit/s`，线上值为 `0x02`。
    Kbps250,
    /// `125 kbit/s`，线上值为 `0x03`。
    Kbps125,
    /// `100 kbit/s`，线上值为 `0x04`。
    Kbps100,
}

/// 说明书定义的七种自动回传模式。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReportingMode {
    /// 查询模式，线上值为 `0x00`。
    Query,
    /// 标准帧自动回传位置，线上值为 `0xaa`。
    StandardPosition,
    /// 标准帧自动回传有符号角速度，线上值为 `0x02`。
    StandardSignedSpeed,
    /// 标准帧自动回传无符号角速度，线上值为 `0x07`。
    StandardUnsignedSpeed,
    /// 扩展帧自动回传位置，线上值为 `0x18`。
    ExtendedPosition,
    /// 扩展帧自动回传有符号角速度，线上值为 `0x12`。
    ExtendedSignedSpeed,
    /// 扩展帧自动回传无符号角速度，线上值为 `0x17`。
    ExtendedUnsignedSpeed,
}

/// 编码器数值递增方向。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    /// 顺时针，线上值为 `0x00`。
    Clockwise,
    /// 逆时针，线上值为 `0x01`。
    Counterclockwise,
}

/// 主机发送的全部功能请求。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Request {
    /// 读取位置值；固定参数为 `0x00`。
    ReadPosition,
    /// 设定新的标准 CAN 地址，范围为 `1..=255`。
    SetStandardAddress(u8),
    /// 设定新的 29 位扩展 CAN 地址，范围为 `0..=0x1fff_ffff`。
    ///
    /// 四字节请求参数使用大端；编码使用 `LEN=7`，解码也接受 `LEN=4, DLC=7`。
    SetExtendedAddress(u32),
    /// 设定 CAN 波特率。
    SetBaudRate(BaudRate),
    /// 设定自动回传模式。
    SetReportingMode(ReportingMode),
    /// 设定自动回传周期，单位为微秒，范围为 `50..=65535`。
    SetReportingPeriod(u16),
    /// 将当前位置设为零点；固定参数为 `0x00`。
    SetZero,
    /// 设定数值递增方向。
    SetDirection(Direction),
    /// 读取角速度值；固定参数为 `0x00`。
    ReadSpeed,
    /// 设定角速度采样时间，单位为毫秒，范围为 `0..=65535`。
    ///
    /// `1000` 在线上编码为大端字节 `0x03, 0xe8`。
    SetSpeedSampleTime(u16),
    /// 将当前位置设为中点；固定参数为 `0x01`。
    SetMidpoint,
    /// 设定当前位置值；线上字段为完整 `u32`，设备量程由调用方检查。
    ///
    /// `74565` 在线上编码为大端字节 `0x00, 0x01, 0x23, 0x45`。
    SetPosition(u32),
    /// 将当前位置设为五圈值；固定参数为 `0x01`。
    SetFiveTurns,
}

/// 可以由普通写入功能返回单字节状态的功能码。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AckCommand {
    /// 确认设定标准地址（`0x02`）。
    SetStandardAddress,
    /// 确认设定波特率（`0x03`）。
    SetBaudRate,
    /// 确认设定自动回传模式（`0x04`）。
    SetReportingMode,
    /// 确认设定自动回传周期（`0x05`）。
    SetReportingPeriod,
    /// 确认设定零点（`0x06`）。
    SetZero,
    /// 确认设定递增方向（`0x07`）。
    SetDirection,
    /// 确认设定角速度采样时间（`0x0b`）。
    SetSpeedSampleTime,
    /// 确认设定中点（`0x0c`）。
    SetMidpoint,
    /// 确认设定当前位置（`0x0d`）。
    SetPosition,
    /// 确认设定五圈值（`0x0f`）。
    SetFiveTurns,
}

/// 普通写入命令的原始八位状态。
///
/// `0` 表示成功，其他值为错误码；未知值原样保留。请求匹配和设备操作结果由调用方处理。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Status(pub u8);

/// 扩展地址设定命令的原始 32 位状态。
///
/// 线上字段为大端，`0` 表示成功，其他值为错误码；未知值原样保留。
/// 请求匹配、地址变更和设备操作结果由调用方处理。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Status32(pub u32);

/// 从设备返回的消息。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Response {
    /// 位置原始值，线上字段为完整 `u32`。
    Position(u32),
    /// 有符号角速度原始值，线上字段为完整 `i32`。
    Speed(i32),
    /// 普通写入功能的单字节状态。
    Ack {
        /// 被确认的普通写入功能。
        command: AckCommand,
        /// 设备返回的原始状态，含义见 [`Status`]。
        status: Status,
    },
    /// 扩展地址设定功能的完整 32 位状态，线上字段为大端。
    SetExtendedAddress(Status32),
}

/// 无法将值表示成 CAN 帧时的错误。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EncodeError {
    /// 29 位扩展 CAN ID 超出 `0..=0x1fff_ffff`。
    InvalidExtendedCanId(u32),
    /// 新标准地址为 `0`，而协议只允许 `1..=255`。
    InvalidStandardAddress(u8),
    /// 自动回传周期小于协议最小值 50 微秒。
    InvalidReportingPeriod(u16),
}

impl fmt::Display for EncodeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidExtendedCanId(value) => {
                write!(formatter, "extended CAN id exceeds 29 bits: {value:#x}")
            }
            Self::InvalidStandardAddress(value) => {
                write!(formatter, "invalid new standard address: {value}")
            }
            Self::InvalidReportingPeriod(value) => {
                write!(
                    formatter,
                    "invalid reporting period in microseconds: {value}"
                )
            }
        }
    }
}

impl core::error::Error for EncodeError {}

/// 无法将输入帧解释为协议消息时的错误。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DecodeError {
    /// 标准 CAN ID 超出 11 位。
    InvalidStandardCanId(u16),
    /// 扩展 CAN ID 超出 29 位。
    InvalidExtendedCanId(u32),
    /// 收到协议不接受的 Classic RTR 帧。
    UnsupportedRemoteFrame,
    /// 收到协议不接受的 CAN FD 帧。
    UnsupportedCanFd,
    /// 已匹配 ID 的 Classic 数据帧超过八字节。
    DataLengthExceedsClassicCan(usize),
    /// 已匹配 ID 的数据帧没有长度、设备 ID 和功能码三个基本头字段。
    IncompleteHeader {
        /// 实际数据帧字节数。
        actual: usize,
    },
    /// 长度字段不等于实际有效数据字节数。
    LengthMismatch {
        /// 报文第一个字节中的长度。
        declared: u8,
        /// 由帧 DLC 表示的实际有效数据长度。
        actual: usize,
    },
    /// 报文长度字段超过 Classic CAN 的八字节上限。
    DeclaredLengthExceedsClassicCan(u8),
    /// 已匹配 ID 的数据域设备 ID 不等于显式目标地址。
    DeviceIdMismatch {
        /// 由 [`Address`] 提供的目标设备 ID。
        expected: u8,
        /// 报文中的设备 ID。
        actual: u8,
    },
    /// 已知功能的报文长度不符合该功能在当前方向的定义。
    InvalidLength {
        /// 功能码。
        function: u8,
        /// 定义的完整报文长度。
        expected: u8,
        /// 实际完整报文长度。
        actual: u8,
    },
    /// 固定请求参数不符合定义。
    InvalidFixedParameter {
        /// 功能码。
        function: u8,
        /// 定义的固定参数。
        expected: u8,
        /// 报文中的参数。
        actual: u8,
    },
    /// 新标准地址为 `0`，而协议只允许 `1..=255`。
    InvalidStandardAddress(u8),
    /// 未定义的波特率原始值。
    InvalidBaudRate(u8),
    /// 未定义的自动回传模式原始值。
    InvalidReportingMode(u8),
    /// 未定义的方向原始值。
    InvalidDirection(u8),
    /// 自动回传周期小于协议最小值 50 微秒。
    InvalidReportingPeriod(u16),
}

impl fmt::Display for DecodeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidStandardCanId(value) => {
                write!(formatter, "standard CAN id exceeds 11 bits: {value:#x}")
            }
            Self::InvalidExtendedCanId(value) => {
                write!(formatter, "extended CAN id exceeds 29 bits: {value:#x}")
            }
            Self::UnsupportedRemoteFrame => formatter.write_str("Classic CAN RTR is unsupported"),
            Self::UnsupportedCanFd => formatter.write_str("CAN FD is unsupported"),
            Self::DataLengthExceedsClassicCan(value) => {
                write!(formatter, "Classic CAN data exceeds eight bytes: {value}")
            }
            Self::IncompleteHeader { actual } => {
                write!(
                    formatter,
                    "BRT protocol header needs three bytes, got {actual}"
                )
            }
            Self::LengthMismatch { declared, actual } => write!(
                formatter,
                "BRT protocol length mismatch: declared {declared}, actual {actual}"
            ),
            Self::DeclaredLengthExceedsClassicCan(value) => {
                write!(
                    formatter,
                    "declared Classic CAN length exceeds eight bytes: {value}"
                )
            }
            Self::DeviceIdMismatch { expected, actual } => {
                write!(
                    formatter,
                    "BRT device id mismatch: expected {expected}, got {actual}"
                )
            }
            Self::InvalidLength {
                function,
                expected,
                actual,
            } => write!(
                formatter,
                "invalid length for BRT function {function:#04x}: expected {expected}, got {actual}"
            ),
            Self::InvalidFixedParameter {
                function,
                expected,
                actual,
            } => write!(
                formatter,
                "invalid fixed parameter for BRT function {function:#04x}: expected {expected:#04x}, got {actual:#04x}"
            ),
            Self::InvalidStandardAddress(value) => {
                write!(formatter, "invalid new standard address: {value}")
            }
            Self::InvalidBaudRate(value) => {
                write!(formatter, "invalid BRT baud rate: {value:#04x}")
            }
            Self::InvalidReportingMode(value) => {
                write!(formatter, "invalid BRT reporting mode: {value:#04x}")
            }
            Self::InvalidDirection(value) => {
                write!(formatter, "invalid BRT direction: {value:#04x}")
            }
            Self::InvalidReportingPeriod(value) => {
                write!(
                    formatter,
                    "invalid reporting period in microseconds: {value}"
                )
            }
        }
    }
}

impl core::error::Error for DecodeError {}

/// 将一个主机请求编码为 Classic CAN 数据帧。
///
/// 返回包含完整地址和有效数据的帧；参数超出协议范围时返回 [`EncodeError`]。
/// `address` 由调用方指定，帧发送由调用方执行。
pub fn encode_request(address: Address, request: Request) -> Result<EncodedFrame, EncodeError> {
    let mut data = [0; 8];
    let (function, length) = match request {
        Request::ReadPosition => {
            data[3] = 0;
            (READ_POSITION, 4)
        }
        Request::SetStandardAddress(value) => {
            if value == 0 {
                return Err(EncodeError::InvalidStandardAddress(value));
            }
            data[3] = value;
            (SET_STANDARD_ADDRESS, 4)
        }
        Request::SetExtendedAddress(value) => {
            if value > MAX_EXTENDED_ID {
                return Err(EncodeError::InvalidExtendedCanId(value));
            }
            data[3..7].copy_from_slice(&value.to_be_bytes());
            (SET_EXTENDED_ADDRESS, 7)
        }
        Request::SetBaudRate(value) => {
            data[3] = baud_rate_byte(value);
            (SET_BAUD_RATE, 4)
        }
        Request::SetReportingMode(value) => {
            data[3] = reporting_mode_byte(value);
            (SET_REPORTING_MODE, 4)
        }
        Request::SetReportingPeriod(value) => {
            if value < 50 {
                return Err(EncodeError::InvalidReportingPeriod(value));
            }
            put_u16(&mut data, 3, value);
            (SET_REPORTING_PERIOD, 5)
        }
        Request::SetZero => {
            data[3] = 0;
            (SET_ZERO, 4)
        }
        Request::SetDirection(value) => {
            data[3] = direction_byte(value);
            (SET_DIRECTION, 4)
        }
        Request::ReadSpeed => {
            data[3] = 0;
            (READ_SPEED, 4)
        }
        Request::SetSpeedSampleTime(value) => {
            data[3..5].copy_from_slice(&value.to_be_bytes());
            (SET_SPEED_SAMPLE_TIME, 5)
        }
        Request::SetMidpoint => {
            data[3] = 1;
            (SET_MIDPOINT, 4)
        }
        Request::SetPosition(value) => {
            data[3..7].copy_from_slice(&value.to_be_bytes());
            (SET_POSITION, 7)
        }
        Request::SetFiveTurns => {
            data[3] = 1;
            (SET_FIVE_TURNS, 4)
        }
    };
    data[0] = length;
    data[1] = address.device_id();
    data[2] = function;
    Ok(EncodedFrame::new(address.id(), data, length))
}

/// 将一个设备响应编码为 Classic CAN 数据帧。
///
/// 使用调用方指定的 CAN ID 和设备 ID，按字段端序编码原始状态值。
/// 设址应答使用的地址由调用方通过 `address` 指定。
pub fn encode_response(address: Address, response: Response) -> Result<EncodedFrame, EncodeError> {
    let mut data = [0; 8];
    let (function, length) = match response {
        Response::Position(value) => {
            put_u32(&mut data, 3, value);
            (READ_POSITION, 7)
        }
        Response::Speed(value) => {
            put_u32(&mut data, 3, value as u32);
            (READ_SPEED, 7)
        }
        Response::Ack { command, status } => {
            data[3] = status.0;
            (ack_command_byte(command), 4)
        }
        Response::SetExtendedAddress(status) => {
            data[3..7].copy_from_slice(&status.0.to_be_bytes());
            (SET_EXTENDED_ADDRESS, 7)
        }
    };
    data[0] = length;
    data[1] = address.device_id();
    data[2] = function;
    Ok(EncodedFrame::new(address.id(), data, length))
}

/// 将一个借用帧按主机请求方向解码。
///
/// 先校验 ID 格式，并拒绝任意 RTR 或 CAN FD（包括其他设备的帧）。合法但 ID 不匹配的
/// Classic 数据帧返回 `Ok(None)`；已匹配 ID 的帧先检查八字节上限、基本头、`LEN` 与实际
/// DLC、设备 ID，随后才对未知功能返回 `Ok(None)`。已知功能不得接受补齐到八字节的尾部。
/// 仅 `0x22` 请求允许 `LEN=4, DLC=7`，与 `LEN=7, DLC=7` 解码为同一个请求。
pub fn decode_request(
    address: Address,
    frame: FrameRef<'_>,
) -> Result<Option<Request>, DecodeError> {
    let data = matched_data(address, frame, true)?;
    let Some(data) = data else {
        return Ok(None);
    };
    let function = data[2];
    let request = match function {
        READ_POSITION => {
            exact_length(data, function, 4)?;
            fixed_parameter(data, function, 0)?;
            Request::ReadPosition
        }
        SET_STANDARD_ADDRESS => {
            exact_length(data, function, 4)?;
            if data[3] == 0 {
                return Err(DecodeError::InvalidStandardAddress(0));
            }
            Request::SetStandardAddress(data[3])
        }
        SET_EXTENDED_ADDRESS => {
            exact_length(data, function, 7)?;
            let value = u32::from_be_bytes([data[3], data[4], data[5], data[6]]);
            if value > MAX_EXTENDED_ID {
                return Err(DecodeError::InvalidExtendedCanId(value));
            }
            Request::SetExtendedAddress(value)
        }
        SET_BAUD_RATE => {
            exact_length(data, function, 4)?;
            Request::SetBaudRate(parse_baud_rate(data[3])?)
        }
        SET_REPORTING_MODE => {
            exact_length(data, function, 4)?;
            Request::SetReportingMode(parse_reporting_mode(data[3])?)
        }
        SET_REPORTING_PERIOD => {
            exact_length(data, function, 5)?;
            let value = read_u16(data, 3);
            if value < 50 {
                return Err(DecodeError::InvalidReportingPeriod(value));
            }
            Request::SetReportingPeriod(value)
        }
        SET_ZERO => {
            exact_length(data, function, 4)?;
            fixed_parameter(data, function, 0)?;
            Request::SetZero
        }
        SET_DIRECTION => {
            exact_length(data, function, 4)?;
            Request::SetDirection(parse_direction(data[3])?)
        }
        READ_SPEED => {
            exact_length(data, function, 4)?;
            fixed_parameter(data, function, 0)?;
            Request::ReadSpeed
        }
        SET_SPEED_SAMPLE_TIME => {
            exact_length(data, function, 5)?;
            Request::SetSpeedSampleTime(u16::from_be_bytes([data[3], data[4]]))
        }
        SET_MIDPOINT => {
            exact_length(data, function, 4)?;
            fixed_parameter(data, function, 1)?;
            Request::SetMidpoint
        }
        SET_POSITION => {
            exact_length(data, function, 7)?;
            Request::SetPosition(u32::from_be_bytes([data[3], data[4], data[5], data[6]]))
        }
        SET_FIVE_TURNS => {
            exact_length(data, function, 4)?;
            fixed_parameter(data, function, 1)?;
            Request::SetFiveTurns
        }
        _ => return Ok(None),
    };
    Ok(Some(request))
}

/// 将一个借用帧按设备响应方向解码。
///
/// 位置、速度和状态均保留完整线上位模式。位置和有符号速度分别按 `0x01`／`0x0a`
/// 布局解析；返回值不区分查询应答与主动回传，无符号速度回传布局未实现。
/// ID、帧形态、基本头、`LEN`、DLC 和设备 ID 的过滤顺序与 [`decode_request`] 相同；因此
/// 无关 RTR／FD 仍会被拒绝，未知功能只会在完整头部已验证后返回 `Ok(None)`。
/// 应答无请求方向的长度例外，`0x22` 应答必须为 `LEN=7, DLC=7`。
pub fn decode_response(
    address: Address,
    frame: FrameRef<'_>,
) -> Result<Option<Response>, DecodeError> {
    let data = matched_data(address, frame, false)?;
    let Some(data) = data else {
        return Ok(None);
    };
    let function = data[2];
    let response = match function {
        READ_POSITION => {
            exact_length(data, function, 7)?;
            Response::Position(read_u32(data, 3))
        }
        READ_SPEED => {
            exact_length(data, function, 7)?;
            Response::Speed(read_u32(data, 3) as i32)
        }
        SET_EXTENDED_ADDRESS => {
            exact_length(data, function, 7)?;
            Response::SetExtendedAddress(Status32(u32::from_be_bytes([
                data[3], data[4], data[5], data[6],
            ])))
        }
        _ => {
            let Some(command) = parse_ack_command(function) else {
                return Ok(None);
            };
            exact_length(data, function, 4)?;
            Response::Ack {
                command,
                status: Status(data[3]),
            }
        }
    };
    Ok(Some(response))
}

fn matched_data<'a>(
    address: Address,
    frame: FrameRef<'a>,
    is_request: bool,
) -> Result<Option<&'a [u8]>, DecodeError> {
    match frame.id {
        FrameId::Standard(id) if id > MAX_STANDARD_ID => {
            return Err(DecodeError::InvalidStandardCanId(id));
        }
        FrameId::Extended(id) if id > MAX_EXTENDED_ID => {
            return Err(DecodeError::InvalidExtendedCanId(id));
        }
        FrameId::Standard(_) | FrameId::Extended(_) => {}
    }
    let data = match frame.payload {
        FramePayload::Remote { .. } => return Err(DecodeError::UnsupportedRemoteFrame),
        FramePayload::Fd(_) => return Err(DecodeError::UnsupportedCanFd),
        FramePayload::Data(data) => data,
    };
    if frame.id != address.id() {
        return Ok(None);
    }
    if data.len() > 8 {
        return Err(DecodeError::DataLengthExceedsClassicCan(data.len()));
    }
    if data.len() < 3 {
        return Err(DecodeError::IncompleteHeader { actual: data.len() });
    }
    let declared = data[0];
    if declared > 8 {
        return Err(DecodeError::DeclaredLengthExceedsClassicCan(declared));
    }
    // 0x22 请求的 LEN 可填 4 或 7；实际 DLC 始终要求 7。
    let extended_request_length =
        is_request && data[2] == SET_EXTENDED_ADDRESS && declared == 4 && data.len() == 7;
    if usize::from(declared) != data.len() && !extended_request_length {
        return Err(DecodeError::LengthMismatch {
            declared,
            actual: data.len(),
        });
    }
    if data[1] != address.device_id() {
        return Err(DecodeError::DeviceIdMismatch {
            expected: address.device_id(),
            actual: data[1],
        });
    }
    Ok(Some(data))
}

fn exact_length(data: &[u8], function: u8, expected: u8) -> Result<(), DecodeError> {
    let actual = data.len() as u8;
    if actual == expected {
        Ok(())
    } else {
        Err(DecodeError::InvalidLength {
            function,
            expected,
            actual,
        })
    }
}

fn fixed_parameter(data: &[u8], function: u8, expected: u8) -> Result<(), DecodeError> {
    if data[3] == expected {
        Ok(())
    } else {
        Err(DecodeError::InvalidFixedParameter {
            function,
            expected,
            actual: data[3],
        })
    }
}

fn baud_rate_byte(value: BaudRate) -> u8 {
    match value {
        BaudRate::Kbps500 => 0,
        BaudRate::Mbps1 => 1,
        BaudRate::Kbps250 => 2,
        BaudRate::Kbps125 => 3,
        BaudRate::Kbps100 => 4,
    }
}

fn parse_baud_rate(value: u8) -> Result<BaudRate, DecodeError> {
    match value {
        0 => Ok(BaudRate::Kbps500),
        1 => Ok(BaudRate::Mbps1),
        2 => Ok(BaudRate::Kbps250),
        3 => Ok(BaudRate::Kbps125),
        4 => Ok(BaudRate::Kbps100),
        _ => Err(DecodeError::InvalidBaudRate(value)),
    }
}

fn reporting_mode_byte(value: ReportingMode) -> u8 {
    match value {
        ReportingMode::Query => 0,
        ReportingMode::StandardPosition => 0xaa,
        ReportingMode::StandardSignedSpeed => 2,
        ReportingMode::StandardUnsignedSpeed => 7,
        ReportingMode::ExtendedPosition => 0x18,
        ReportingMode::ExtendedSignedSpeed => 0x12,
        ReportingMode::ExtendedUnsignedSpeed => 0x17,
    }
}

fn parse_reporting_mode(value: u8) -> Result<ReportingMode, DecodeError> {
    match value {
        0 => Ok(ReportingMode::Query),
        0xaa => Ok(ReportingMode::StandardPosition),
        2 => Ok(ReportingMode::StandardSignedSpeed),
        7 => Ok(ReportingMode::StandardUnsignedSpeed),
        0x18 => Ok(ReportingMode::ExtendedPosition),
        0x12 => Ok(ReportingMode::ExtendedSignedSpeed),
        0x17 => Ok(ReportingMode::ExtendedUnsignedSpeed),
        _ => Err(DecodeError::InvalidReportingMode(value)),
    }
}

fn direction_byte(value: Direction) -> u8 {
    match value {
        Direction::Clockwise => 0,
        Direction::Counterclockwise => 1,
    }
}

fn parse_direction(value: u8) -> Result<Direction, DecodeError> {
    match value {
        0 => Ok(Direction::Clockwise),
        1 => Ok(Direction::Counterclockwise),
        _ => Err(DecodeError::InvalidDirection(value)),
    }
}

fn ack_command_byte(value: AckCommand) -> u8 {
    match value {
        AckCommand::SetStandardAddress => SET_STANDARD_ADDRESS,
        AckCommand::SetBaudRate => SET_BAUD_RATE,
        AckCommand::SetReportingMode => SET_REPORTING_MODE,
        AckCommand::SetReportingPeriod => SET_REPORTING_PERIOD,
        AckCommand::SetZero => SET_ZERO,
        AckCommand::SetDirection => SET_DIRECTION,
        AckCommand::SetSpeedSampleTime => SET_SPEED_SAMPLE_TIME,
        AckCommand::SetMidpoint => SET_MIDPOINT,
        AckCommand::SetPosition => SET_POSITION,
        AckCommand::SetFiveTurns => SET_FIVE_TURNS,
    }
}

fn parse_ack_command(value: u8) -> Option<AckCommand> {
    match value {
        SET_STANDARD_ADDRESS => Some(AckCommand::SetStandardAddress),
        SET_BAUD_RATE => Some(AckCommand::SetBaudRate),
        SET_REPORTING_MODE => Some(AckCommand::SetReportingMode),
        SET_REPORTING_PERIOD => Some(AckCommand::SetReportingPeriod),
        SET_ZERO => Some(AckCommand::SetZero),
        SET_DIRECTION => Some(AckCommand::SetDirection),
        SET_SPEED_SAMPLE_TIME => Some(AckCommand::SetSpeedSampleTime),
        SET_MIDPOINT => Some(AckCommand::SetMidpoint),
        SET_POSITION => Some(AckCommand::SetPosition),
        SET_FIVE_TURNS => Some(AckCommand::SetFiveTurns),
        _ => None,
    }
}

fn put_u16(data: &mut [u8; 8], index: usize, value: u16) {
    let bytes = value.to_le_bytes();
    data[index] = bytes[0];
    data[index + 1] = bytes[1];
}

fn put_u32(data: &mut [u8; 8], index: usize, value: u32) {
    let bytes = value.to_le_bytes();
    data[index] = bytes[0];
    data[index + 1] = bytes[1];
    data[index + 2] = bytes[2];
    data[index + 3] = bytes[3];
}

fn read_u16(data: &[u8], index: usize) -> u16 {
    u16::from_le_bytes([data[index], data[index + 1]])
}

fn read_u32(data: &[u8], index: usize) -> u32 {
    u32::from_le_bytes([
        data[index],
        data[index + 1],
        data[index + 2],
        data[index + 3],
    ])
}
