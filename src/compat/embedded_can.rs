// Copyright The odrive-can-protocol Contributors
// Copyright The brt-can-protocol Contributors

//! `embedded-can` 0.4 的 Classic CAN 帧转换。
//!
//! 此 trait 不携带 CAN FD 或总线错误标志。接收 Embassy FDCAN 等可能包含 CAN FD 的
//! 原生类型时，必须使用其专用适配，而不是根据数据长度调用本模块。

use embedded_can::{Frame, Id, StandardId};

use crate::{EncodedFrame, FrameId, FramePayload, FrameRef};

impl EncodedFrame {
    /// 将编码的 Classic 数据帧转换为一个 `embedded_can::Frame` 实现。
    ///
    /// 编码结果本身总是 Classic 数据帧。调用方必须选择其 `F::new` 确实构造 Classic
    /// 数据帧的类型；`embedded-can::Frame` trait 不携带 FDF，不能保证此条件。`None`
    /// 表示目标帧类型拒绝了已给标识符或有效数据；本库不替调用方重试，也不执行发送。
    pub fn to_embedded_can<F: Frame>(&self) -> Option<F> {
        F::new(embedded_can_id(self.id())?, self.data())
    }
}

fn embedded_can_id(id: FrameId) -> Option<Id> {
    match id {
        FrameId::Standard(raw) => StandardId::new(raw).map(Into::into),
        FrameId::Extended(raw) => embedded_can::ExtendedId::new(raw).map(Into::into),
    }
}

/// Classic CAN 的 DLC 或驱动数据缓冲不符合 `embedded-can::Frame` 合同。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InvalidFrameLength;

impl core::fmt::Display for InvalidFrameLength {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str("invalid Classic CAN frame length")
    }
}

impl core::error::Error for InvalidFrameLength {}

impl<'a> FrameRef<'a> {
    /// 从已确认是 Classic 数据帧或 RTR 的 `embedded-can` 帧借用解码输入。
    ///
    /// 调用方必须先排除 CAN FD 和总线错误，因为通用 trait 不提供这两类语义。数据帧
    /// 的 DLC 必须为 `0..=8`，且驱动提供的数据至少含 DLC 个字节；本方法只借用其
    /// 有效前缀。RTR 保留 DLC，不读取其数据缓冲。
    pub fn from_classic_embedded_can<F: Frame>(frame: &'a F) -> Result<Self, InvalidFrameLength> {
        let dlc = frame.dlc();
        if dlc > 8 {
            return Err(InvalidFrameLength);
        }

        let payload = if frame.is_remote_frame() {
            FramePayload::Remote { dlc: dlc as u8 }
        } else {
            FramePayload::Data(frame.data().get(..dlc).ok_or(InvalidFrameLength)?)
        };

        Ok(Self {
            id: frame.id().into(),
            payload,
        })
    }
}

impl From<Id> for FrameId {
    fn from(id: Id) -> Self {
        match id {
            Id::Standard(id) => Self::Standard(id.as_raw()),
            Id::Extended(id) => Self::Extended(id.as_raw()),
        }
    }
}
