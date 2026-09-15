// Copyright The odrive-can-protocol Contributors
// Copyright The brt-can-protocol Contributors

//! Embassy STM32 0.6 原生帧转换。
//!
//! `EncodedFrame` 转为 `Frame` 或 `FdFrame` 时始终保持 Classic 数据帧形态；`FdFrame`
//! 仅是可容纳该帧的容器，并不表示 FDF 已置位。接收转换保留实际 FDF，即使 CAN FD
//! 有效载荷不超过八字节；协议层据此决定是否接受。

use embassy_stm32::can::frame::{FdFrame, Frame, Header};
use embedded_can::{ExtendedId, Id, StandardId};

use crate::{EncodedFrame, FrameId, FramePayload, FrameRef};

impl From<&EncodedFrame> for Frame {
    /// 从已验证的协议编码结果构造 Classic `Frame`。
    fn from(frame: &EncodedFrame) -> Self {
        Self::new(encoded_header(frame), frame.data())
            .expect("protocol encoder produced a valid Classic CAN frame")
    }
}

impl From<&EncodedFrame> for FdFrame {
    /// 从已验证的协议编码结果构造 Classic `FdFrame` 容器。
    fn from(frame: &EncodedFrame) -> Self {
        Self::new(encoded_header(frame), frame.data())
            .expect("protocol encoder produced a valid Classic CAN frame")
    }
}

fn encoded_header(frame: &EncodedFrame) -> Header {
    Header::new(encoded_id(frame.id()), frame.dlc(), false)
}

fn encoded_id(id: FrameId) -> Id {
    match id {
        FrameId::Standard(raw) => StandardId::new(raw)
            .expect("protocol encoder produced a valid standard CAN ID")
            .into(),
        FrameId::Extended(raw) => ExtendedId::new(raw)
            .expect("protocol encoder produced a valid extended CAN ID")
            .into(),
    }
}

/// 原生帧头部声明的长度不能由其缓冲安全且准确地表示。
///
/// 此错误覆盖 Classic DLC 大于八、CAN FD 长度不是标准 CAN FD 长度集合，或头部长度
/// 超出原生帧缓冲的情形。适配错误与协议 `DecodeError` 分开：调用方应先处理本错误，
/// 仅将成功的 [`FrameRef`] 交给协议解码器。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FromEmbassyError {
    /// 头部长度与 Classic／CAN FD 合同或帧缓冲不相容。
    InvalidPayloadLength,
}

impl core::fmt::Display for FromEmbassyError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str("Embassy CAN header length cannot be represented by its frame buffer")
    }
}

impl core::error::Error for FromEmbassyError {}

impl<'a> TryFrom<&'a Frame> for FrameRef<'a> {
    type Error = FromEmbassyError;

    /// 借用 Classic 帧容器中的实际帧形态。
    fn try_from(frame: &'a Frame) -> Result<Self, Self::Error> {
        frame_ref_from_parts(frame.header(), frame.raw_data())
    }
}

impl<'a> TryFrom<&'a FdFrame> for FrameRef<'a> {
    type Error = FromEmbassyError;

    /// 借用 FDCAN 帧容器中的实际帧形态。
    fn try_from(frame: &'a FdFrame) -> Result<Self, Self::Error> {
        let header = frame.header();
        validate_header_length(header)?;

        if !header.fdcan() && header.rtr() {
            return frame_ref_from_valid_parts(header, &[]);
        }

        // 已先校验 `len <= 64` 与 CAN FD 规范长度，`FdFrame::data` 不会越界。
        frame_ref_from_valid_parts(header, frame.data())
    }
}

fn frame_ref_from_parts<'a>(
    header: &Header,
    raw_data: &'a [u8],
) -> Result<FrameRef<'a>, FromEmbassyError> {
    validate_header_length(header)?;
    frame_ref_from_valid_parts(header, raw_data)
}

fn frame_ref_from_valid_parts<'a>(
    header: &Header,
    raw_data: &'a [u8],
) -> Result<FrameRef<'a>, FromEmbassyError> {
    let len = usize::from(header.len());
    let payload = if header.fdcan() {
        // FDF 必须优先于 RTR；长度小于等于八不能推断为 Classic CAN。
        FramePayload::Fd(
            raw_data
                .get(..len)
                .ok_or(FromEmbassyError::InvalidPayloadLength)?,
        )
    } else if header.rtr() {
        FramePayload::Remote { dlc: header.len() }
    } else {
        FramePayload::Data(
            raw_data
                .get(..len)
                .ok_or(FromEmbassyError::InvalidPayloadLength)?,
        )
    };

    Ok(FrameRef {
        id: (*header.id()).into(),
        payload,
    })
}

fn validate_header_length(header: &Header) -> Result<(), FromEmbassyError> {
    let len = header.len();
    if header.fdcan() {
        if is_canonical_fd_length(len) {
            Ok(())
        } else {
            Err(FromEmbassyError::InvalidPayloadLength)
        }
    } else if len <= 8 {
        Ok(())
    } else {
        Err(FromEmbassyError::InvalidPayloadLength)
    }
}

const fn is_canonical_fd_length(len: u8) -> bool {
    matches!(len, 0..=8 | 12 | 16 | 20 | 24 | 32 | 48 | 64)
}
