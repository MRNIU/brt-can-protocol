// Copyright The odrive-can-protocol Contributors
// Copyright The brt-can-protocol Contributors

//! 驱动无关的帧模型，保留标识符格式和实际帧形态。
//!
//! 借鉴 odrive-can-protocol 的 MIT 帧模型；输入借用有效数据，输出使用八字节缓冲。

/// CAN 标识符及其格式；解码器负责校验原始值是否在格式范围内。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrameId {
    /// 11 位标准 CAN 标识符。
    Standard(u16),
    /// 29 位扩展 CAN 标识符。
    Extended(u32),
}

/// 借用的实际帧形态，不能通过数据长度推断是否为 CAN FD。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FramePayload<'a> {
    /// Classic 数据帧的有效字节；切片长度就是 DLC 对应的数据字节数。
    ///
    /// 调用方必须先核对原生 DLC 与缓冲容量，只传有效前缀，不传未使用的缓冲尾部。
    Data(&'a [u8]),
    /// Classic RTR；BRT 协议解码器拒绝此类帧。
    Remote {
        /// 原生远程帧报告的 DLC。
        dlc: u8,
    },
    /// CAN FD 数据帧；即使不超过八字节也必须保留此变体，BRT 协议拒绝它。
    Fd(&'a [u8]),
}

/// 借用驱动数据的解码输入视图，不拥有或改变设备状态。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FrameRef<'a> {
    /// CAN 标识符，标准与扩展 ID 即使数值相同也不同。
    pub id: FrameId,
    /// 实际帧形态和有效数据。
    pub payload: FramePayload<'a>,
}

/// 固定容量的 Classic CAN 数据帧编码结果。
///
/// 只有协议编码器能构造此类型，保证 ID 合法、DLC 不超过八字节；不分配内存。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EncodedFrame {
    id: FrameId,
    data: [u8; 8],
    len: u8,
}

impl EncodedFrame {
    pub(crate) const fn new(id: FrameId, data: [u8; 8], len: u8) -> Self {
        Self { id, data, len }
    }

    /// 返回完整 CAN ID 与标准／扩展格式。
    pub const fn id(&self) -> FrameId {
        self.id
    }

    /// 返回有效数据，不包含固定缓冲中的未使用尾部。
    pub fn data(&self) -> &[u8] {
        &self.data[..usize::from(self.len)]
    }

    /// 返回 Classic 数据帧 DLC，等于有效数据字节数。
    pub const fn dlc(&self) -> u8 {
        self.len
    }

    /// 借用为解码输入；编码结果始终为 Classic 数据帧。
    pub fn as_ref(&self) -> FrameRef<'_> {
        FrameRef {
            id: self.id,
            payload: FramePayload::Data(self.data()),
        }
    }
}
