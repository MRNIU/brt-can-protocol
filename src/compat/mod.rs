// Copyright The brt-can-protocol Contributors

//! 可选的原生 CAN 帧转换；只保留帧表示，不访问 CAN 外设。

/// Embassy STM32 0.6 原生 `Frame` 与 `FdFrame` 的转换。
#[cfg(feature = "embassy-stm32")]
pub mod embassy;
/// `embedded-can` 0.4 的 Classic CAN 帧转换。
pub mod embedded_can;
