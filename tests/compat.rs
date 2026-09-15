// Copyright The brt-can-protocol Contributors
//! 验证可选帧适配保留标识符、RTR、FDF 与有效字节范围，不执行设备 I/O。

#![cfg(feature = "embedded-can")]

use brt_can_protocol::protocol::{Address, Request, encode_request};
use brt_can_protocol::{FrameId, FramePayload, FrameRef};
use embedded_can::{Frame, Id, StandardId};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FakeFrame<const REJECT_CONSTRUCTION: bool> {
    id: Id,
    data: [u8; 8],
    available: usize,
    dlc: usize,
    remote: bool,
}

impl<const REJECT_CONSTRUCTION: bool> FakeFrame<REJECT_CONSTRUCTION> {
    fn malformed(id: Id, data: [u8; 8], available: usize, dlc: usize, remote: bool) -> Self {
        Self {
            id,
            data,
            available,
            dlc,
            remote,
        }
    }
}

impl<const REJECT_CONSTRUCTION: bool> Frame for FakeFrame<REJECT_CONSTRUCTION> {
    fn new(id: impl Into<Id>, data: &[u8]) -> Option<Self> {
        if REJECT_CONSTRUCTION || data.len() > 8 {
            return None;
        }
        let mut bytes = [0; 8];
        bytes[..data.len()].copy_from_slice(data);
        Some(Self::malformed(
            id.into(),
            bytes,
            data.len(),
            data.len(),
            false,
        ))
    }

    fn new_remote(id: impl Into<Id>, dlc: usize) -> Option<Self> {
        (!REJECT_CONSTRUCTION && dlc <= 8).then(|| Self::malformed(id.into(), [0; 8], 0, dlc, true))
    }

    fn is_extended(&self) -> bool {
        matches!(self.id, Id::Extended(_))
    }

    fn is_remote_frame(&self) -> bool {
        self.remote
    }

    fn id(&self) -> Id {
        self.id
    }

    fn dlc(&self) -> usize {
        self.dlc
    }

    fn data(&self) -> &[u8] {
        &self.data[..self.available]
    }
}

fn encoded_read_position() -> brt_can_protocol::EncodedFrame {
    encode_request(Address::standard(1), Request::ReadPosition).unwrap()
}

#[test]
fn embedded_can_encodes_brt_identity_and_propagates_constructor_rejection() {
    let standard: FakeFrame<false> = encoded_read_position().to_embedded_can().unwrap();
    assert_eq!(standard.id(), Id::Standard(StandardId::new(1).unwrap()));
    assert_eq!(standard.data(), &[4, 1, 1, 0]);
    assert_eq!(standard.dlc(), 4);
    assert!(!standard.is_remote_frame());
    assert!(
        encoded_read_position()
            .to_embedded_can::<FakeFrame<true>>()
            .is_none()
    );

    let extended: FakeFrame<false> = encode_request(
        Address::extended(0x18ff_f225, 0x25).unwrap(),
        Request::ReadPosition,
    )
    .unwrap()
    .to_embedded_can()
    .unwrap();
    assert_eq!(
        extended.id(),
        Id::Extended(embedded_can::ExtendedId::new(0x18ff_f225).unwrap())
    );
    assert_eq!(extended.data(), &[4, 0x25, 1, 0]);
}

#[test]
fn embedded_can_preserves_identity_rtr_dlc_and_adapter_length_errors() {
    let standard = FakeFrame::<false>::new(StandardId::new(1).unwrap(), &[4, 1, 1, 0]).unwrap();
    assert_eq!(
        FrameRef::from_classic_embedded_can(&standard).unwrap(),
        FrameRef {
            id: FrameId::Standard(1),
            payload: FramePayload::Data(&[4, 1, 1, 0]),
        }
    );
    let extended = FakeFrame::<false>::new(
        embedded_can::ExtendedId::new(0x18ff_f225).unwrap(),
        &[4, 0x25, 1, 0],
    )
    .unwrap();
    assert_eq!(
        FrameRef::from_classic_embedded_can(&extended).unwrap().id,
        FrameId::Extended(0x18ff_f225)
    );
    let remote = FakeFrame::<false>::new_remote(StandardId::new(1).unwrap(), 4).unwrap();
    assert_eq!(
        FrameRef::from_classic_embedded_can(&remote)
            .unwrap()
            .payload,
        FramePayload::Remote { dlc: 4 }
    );

    let id = StandardId::new(1).unwrap().into();
    let with_tail =
        FakeFrame::<false>::malformed(id, [4, 1, 1, 0, 0xaa, 0xbb, 0xcc, 0xdd], 8, 4, false);
    assert_eq!(
        FrameRef::from_classic_embedded_can(&with_tail)
            .unwrap()
            .payload,
        FramePayload::Data(&[4, 1, 1, 0])
    );
    use brt_can_protocol::compat::embedded_can::InvalidFrameLength;
    for frame in [
        FakeFrame::<false>::malformed(id, [0; 8], 8, 9, false),
        FakeFrame::<false>::malformed(id, [4, 1, 1, 0, 0, 0, 0, 0], 3, 4, false),
    ] {
        assert_eq!(
            FrameRef::from_classic_embedded_can(&frame),
            Err(InvalidFrameLength)
        );
    }
}

#[cfg(feature = "embassy-stm32")]
mod embassy {
    use super::*;
    use brt_can_protocol::compat::embassy::FromEmbassyError;
    use brt_can_protocol::protocol::{DecodeError, decode_request};
    use embassy_stm32::can::frame::{FdFrame, Frame as EmbassyFrame, Header};

    fn standard_id() -> Id {
        StandardId::new(1).unwrap().into()
    }

    #[test]
    fn native_frames_preserve_classic_identity_rtr_and_fd_before_length_classification() {
        let standard =
            EmbassyFrame::new(Header::new(standard_id(), 4, false), &[4, 1, 1, 0]).unwrap();
        assert_eq!(
            FrameRef::try_from(&standard).unwrap(),
            FrameRef {
                id: FrameId::Standard(1),
                payload: FramePayload::Data(&[4, 1, 1, 0]),
            }
        );
        let remote = EmbassyFrame::new(Header::new(standard_id(), 4, true), &[]).unwrap();
        assert_eq!(
            FrameRef::try_from(&remote).unwrap().payload,
            FramePayload::Remote { dlc: 4 }
        );
        let remote = FdFrame::new(Header::new(standard_id(), 4, true), &[]).unwrap();
        assert_eq!(
            FrameRef::try_from(&remote).unwrap().payload,
            FramePayload::Remote { dlc: 4 }
        );
        // 八字节容器也可能带 FDF；只借用有效字节，不能伪装为 Classic。
        let short_fd = EmbassyFrame::new(
            Header::new_fd(standard_id(), 4, false, false),
            &[4, 1, 1, 0],
        )
        .unwrap();
        let view = FrameRef::try_from(&short_fd).unwrap();
        assert_eq!(view.payload, FramePayload::Fd(&[4, 1, 1, 0]));
        assert_eq!(
            decode_request(Address::standard(1), view),
            Err(DecodeError::UnsupportedCanFd)
        );
        let fd = FdFrame::new(Header::new_fd(standard_id(), 8, true, false), &[]).unwrap();
        let view = FrameRef::try_from(&fd).unwrap();
        assert!(matches!(view.payload, FramePayload::Fd(_)));
        assert_eq!(
            decode_request(Address::standard(1), view),
            Err(DecodeError::UnsupportedCanFd)
        );
        let long_fd =
            FdFrame::new(Header::new_fd(standard_id(), 12, false, false), &[0; 12]).unwrap();
        assert_eq!(
            FrameRef::try_from(&long_fd).unwrap().payload,
            FramePayload::Fd(&[0; 12])
        );
    }

    #[test]
    fn encoded_frame_stays_classic_and_invalid_headers_return_adapter_errors() {
        for address in [
            Address::standard(1),
            Address::extended(0x18ff_f225, 0xa6).unwrap(),
        ] {
            let encoded = encode_request(address, Request::ReadPosition).unwrap();
            let classic: EmbassyFrame = (&encoded).into();
            let fd_container: FdFrame = (&encoded).into();
            assert_eq!(FrameRef::try_from(&classic).unwrap(), encoded.as_ref());
            assert_eq!(FrameRef::try_from(&fd_container).unwrap(), encoded.as_ref());
        }

        let invalid_classic = EmbassyFrame::new(Header::new(standard_id(), 9, false), &[]).unwrap();
        assert_eq!(
            FrameRef::try_from(&invalid_classic),
            Err(FromEmbassyError::InvalidPayloadLength)
        );
        for frame in [
            FdFrame::new(Header::new_fd(standard_id(), 9, false, false), &[]).unwrap(),
            FdFrame::new(Header::new_fd(standard_id(), 65, false, false), &[]).unwrap(),
        ] {
            assert_eq!(
                FrameRef::try_from(&frame),
                Err(FromEmbassyError::InvalidPayloadLength)
            );
        }
    }
}
