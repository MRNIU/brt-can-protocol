// Copyright The brt-can-protocol Contributors
//! 验证可选帧适配保留标识符、RTR、FDF 与有效字节范围，不执行设备 I/O。

#![cfg(feature = "embedded-can")]

use brt_can_protocol::v2_7::{Address, Request, encode_request};
use brt_can_protocol::{FrameId, FramePayload, FrameRef};
use embedded_can::{Frame, Id, StandardId};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FakeFrame {
    id: Id,
    data: [u8; 8],
    available: usize,
    dlc: usize,
    remote: bool,
}

impl FakeFrame {
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

impl Frame for FakeFrame {
    fn new(id: impl Into<Id>, data: &[u8]) -> Option<Self> {
        if data.len() > 8 {
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
        (dlc <= 8).then(|| Self::malformed(id.into(), [0; 8], 0, dlc, true))
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

struct RejectingFrame;

impl Frame for RejectingFrame {
    fn new(_id: impl Into<Id>, _data: &[u8]) -> Option<Self> {
        None
    }

    fn new_remote(_id: impl Into<Id>, _dlc: usize) -> Option<Self> {
        None
    }

    fn is_extended(&self) -> bool {
        false
    }

    fn is_remote_frame(&self) -> bool {
        false
    }

    fn id(&self) -> Id {
        StandardId::new(0).unwrap().into()
    }

    fn dlc(&self) -> usize {
        0
    }

    fn data(&self) -> &[u8] {
        &[]
    }
}

fn encoded_read_position() -> brt_can_protocol::EncodedFrame {
    encode_request(Address::standard(1), Request::ReadPosition).unwrap()
}

#[test]
fn embedded_can_encodes_brt_data_frame_and_propagates_constructor_rejection() {
    let encoded = encoded_read_position();
    let native: FakeFrame = encoded.to_embedded_can().unwrap();

    assert_eq!(native.id(), Id::Standard(StandardId::new(1).unwrap()));
    assert_eq!(native.data(), &[4, 1, 1, 0]);
    assert!(!native.is_remote_frame());
    assert_eq!(native.dlc(), 4);
    assert!(encoded.to_embedded_can::<RejectingFrame>().is_none());
}

#[test]
fn embedded_can_encodes_extended_brt_data_frame() {
    let encoded = encode_request(
        Address::extended(0x18ff_f225, 0x25).unwrap(),
        Request::ReadPosition,
    )
    .unwrap();
    let native: FakeFrame = encoded.to_embedded_can().unwrap();

    assert_eq!(
        native.id(),
        Id::Extended(embedded_can::ExtendedId::new(0x18ff_f225).unwrap())
    );
    assert_eq!(native.data(), &[4, 0x25, 1, 0]);
}

#[test]
fn embedded_can_preserves_standard_extended_and_remote_frame_identity() {
    let standard = FakeFrame::new(StandardId::new(1).unwrap(), &[4, 1, 1, 0]).unwrap();
    assert_eq!(
        FrameRef::from_classic_embedded_can(&standard).unwrap(),
        FrameRef {
            id: FrameId::Standard(1),
            payload: FramePayload::Data(&[4, 1, 1, 0]),
        }
    );

    let extended = FakeFrame::new(
        embedded_can::ExtendedId::new(0x18ff_f225).unwrap(),
        &[4, 0x25, 1, 0],
    )
    .unwrap();
    assert_eq!(
        FrameRef::from_classic_embedded_can(&extended).unwrap(),
        FrameRef {
            id: FrameId::Extended(0x18ff_f225),
            payload: FramePayload::Data(&[4, 0x25, 1, 0]),
        }
    );

    let remote = FakeFrame::new_remote(StandardId::new(1).unwrap(), 4).unwrap();
    assert_eq!(
        FrameRef::from_classic_embedded_can(&remote)
            .unwrap()
            .payload,
        FramePayload::Remote { dlc: 4 },
    );

    let long_buffer = FakeFrame::malformed(
        StandardId::new(1).unwrap().into(),
        [4, 1, 1, 0, 0xaa, 0xbb, 0xcc, 0xdd],
        8,
        4,
        false,
    );
    assert_eq!(
        FrameRef::from_classic_embedded_can(&long_buffer)
            .unwrap()
            .payload,
        FramePayload::Data(&[4, 1, 1, 0])
    );
}

#[test]
fn embedded_can_rejects_classic_dlc_or_data_shorter_than_it() {
    use brt_can_protocol::compat::embedded_can::InvalidFrameLength;

    let id = StandardId::new(1).unwrap().into();
    let excessive_dlc = FakeFrame::malformed(id, [0; 8], 8, 9, false);
    assert_eq!(
        FrameRef::from_classic_embedded_can(&excessive_dlc),
        Err(InvalidFrameLength)
    );

    let short_buffer = FakeFrame::malformed(id, [4, 1, 1, 0, 0, 0, 0, 0], 3, 4, false);
    assert_eq!(
        FrameRef::from_classic_embedded_can(&short_buffer),
        Err(InvalidFrameLength)
    );
}

#[cfg(feature = "embassy-stm32")]
mod embassy {
    use super::*;
    use brt_can_protocol::compat::embassy::FromEmbassyError;
    use brt_can_protocol::v2_7::{DecodeError, decode_request};
    use embassy_stm32::can::frame::{FdFrame, Frame as EmbassyFrame, Header};

    fn standard_id() -> Id {
        StandardId::new(1).unwrap().into()
    }

    #[test]
    fn native_frames_preserve_classic_standard_extended_and_remote_forms() {
        let standard =
            EmbassyFrame::new(Header::new(standard_id(), 4, false), &[4, 1, 1, 0]).unwrap();
        assert_eq!(
            FrameRef::try_from(&standard).unwrap(),
            FrameRef {
                id: FrameId::Standard(1),
                payload: FramePayload::Data(&[4, 1, 1, 0]),
            }
        );

        let extended_id = embedded_can::ExtendedId::new(0x18ff_f225).unwrap().into();
        let extended =
            EmbassyFrame::new(Header::new(extended_id, 4, false), &[4, 0x25, 1, 0]).unwrap();
        assert_eq!(
            FrameRef::try_from(&extended).unwrap().id,
            FrameId::Extended(0x18ff_f225)
        );

        let remote = EmbassyFrame::new(Header::new(standard_id(), 4, true), &[]).unwrap();
        assert_eq!(
            FrameRef::try_from(&remote).unwrap().payload,
            FramePayload::Remote { dlc: 4 }
        );
    }

    #[test]
    fn encoded_frame_stays_classic_in_frame_and_fdframe_containers() {
        let encoded = encoded_read_position();
        let classic: EmbassyFrame = (&encoded).into();
        let fd_container: FdFrame = (&encoded).into();

        assert!(!classic.header().fdcan());
        assert!(!fd_container.header().fdcan());
        assert_eq!(
            FrameRef::try_from(&classic).unwrap().payload,
            FramePayload::Data(&[4, 1, 1, 0])
        );
        assert_eq!(
            FrameRef::try_from(&fd_container).unwrap().payload,
            FramePayload::Data(&[4, 1, 1, 0])
        );
    }

    #[test]
    fn fdframe_classic_remote_preserves_dlc_without_reading_its_buffer() {
        let remote = FdFrame::new(Header::new(standard_id(), 4, true), &[]).unwrap();

        assert_eq!(
            FrameRef::try_from(&remote).unwrap().payload,
            FramePayload::Remote { dlc: 4 }
        );
    }

    #[test]
    fn native_fd_is_preserved_before_rtr_and_is_never_classified_by_length() {
        let short_fd =
            FdFrame::new(Header::new_fd(standard_id(), 8, false, false), &[0; 8]).unwrap();
        let short_fd_view = FrameRef::try_from(&short_fd).unwrap();
        assert_eq!(short_fd_view.payload, FramePayload::Fd(&[0; 8]));
        assert_eq!(
            decode_request(Address::standard(1), short_fd_view),
            Err(DecodeError::UnsupportedCanFd)
        );

        let long_fd =
            FdFrame::new(Header::new_fd(standard_id(), 12, false, false), &[0; 12]).unwrap();
        assert_eq!(
            FrameRef::try_from(&long_fd).unwrap().payload,
            FramePayload::Fd(&[0; 12])
        );

        let fd_with_rtr = FdFrame::new(Header::new_fd(standard_id(), 8, true, false), &[]).unwrap();
        assert!(matches!(
            FrameRef::try_from(&fd_with_rtr).unwrap().payload,
            FramePayload::Fd(_)
        ));

        let frame_container_fd = EmbassyFrame::new(
            Header::new_fd(standard_id(), 4, false, false),
            &[9, 8, 7, 6],
        )
        .unwrap();
        assert_eq!(
            FrameRef::try_from(&frame_container_fd).unwrap().payload,
            FramePayload::Fd(&[9, 8, 7, 6])
        );
    }

    #[test]
    fn native_headers_with_invalid_lengths_return_adapter_errors() {
        let classic = EmbassyFrame::new(Header::new(standard_id(), 9, false), &[]).unwrap();
        assert_eq!(
            FrameRef::try_from(&classic),
            Err(FromEmbassyError::InvalidPayloadLength)
        );

        let noncanonical_fd =
            FdFrame::new(Header::new_fd(standard_id(), 9, false, false), &[]).unwrap();
        assert_eq!(
            FrameRef::try_from(&noncanonical_fd),
            Err(FromEmbassyError::InvalidPayloadLength)
        );

        let oversized_fd =
            FdFrame::new(Header::new_fd(standard_id(), 65, false, false), &[]).unwrap();
        assert_eq!(
            FrameRef::try_from(&oversized_fd),
            Err(FromEmbassyError::InvalidPayloadLength)
        );
    }
}
