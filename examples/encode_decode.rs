// Copyright The brt-can-protocol Contributors
// 演示 BRT 位置请求和响应的纯协议编解码，不访问或控制 CAN 设备。

//! 演示位置请求、响应解码及可选 Embassy 帧的纯内存转换。

#[cfg(feature = "embassy-stm32")]
use brt_can_protocol::EncodedFrame;
use brt_can_protocol::protocol::{Address, Request, Response, decode_response, encode_request};
use brt_can_protocol::{FrameId, FramePayload, FrameRef};

fn main() {
    let address = Address::standard(1);
    let request = encode_request(address, Request::ReadPosition).unwrap();
    assert_eq!(request.id(), FrameId::Standard(1));
    assert_eq!(request.data(), &[0x04, 0x01, 0x01, 0x00]);

    let bytes = [0x07, 0x01, 0x01, 0x45, 0x23, 0x01, 0x00];
    let response = FrameRef {
        id: FrameId::Standard(1),
        payload: FramePayload::Data(&bytes),
    };
    assert_eq!(
        decode_response(address, response).unwrap(),
        Some(Response::Position(74_565)),
    );

    #[cfg(feature = "embassy-stm32")]
    show_embassy_conversion(&request);

    println!("请求：ID={:?}, data={:02x?}", request.id(), request.data());
    println!("响应：Position(74565)");
}

#[cfg(feature = "embassy-stm32")]
fn show_embassy_conversion(encoded: &EncodedFrame) {
    use embassy_stm32::can::frame::{FdFrame, Frame};

    let classic = Frame::from(encoded);
    let fd_container = FdFrame::from(encoded);
    assert_eq!(FrameRef::try_from(&classic).unwrap(), encoded.as_ref());
    assert_eq!(FrameRef::try_from(&fd_container).unwrap(), encoded.as_ref());
}
