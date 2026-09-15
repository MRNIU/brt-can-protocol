// Copyright The brt-can-protocol Contributors

//! BRT CAN 协议库入口，提供说明书版本明确的编解码和驱动无关帧类型。
//!
//! 默认仅使用 core；可选适配只转换内存中的帧，不执行设备访问。

#![no_std]
#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![doc = include_str!("../README.md")]

/// 可选原生帧转换。
#[cfg(feature = "embedded-can")]
pub mod compat;
/// 与驱动无关的输入视图和固定容量编码结果。
pub mod frame;
/// BRT CAN 应用层协议，版本依据见 README。
pub mod protocol;

pub use frame::{EncodedFrame, FrameId, FramePayload, FrameRef};
