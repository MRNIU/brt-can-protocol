// Copyright The brt-can-protocol Contributors
// 本文件以说明书中的独立字节验证 BRT CAN 协议 v2.7 编解码与拒绝规则。

//! BRT CAN 协议 v2.7 的给定字节测试。

use brt_can_protocol::v2_7::{
    AckCommand, Address, BaudRate, DecodeError, Direction, EncodeError, ReportingMode, Request,
    Response, Status, Status32, decode_request, decode_response, encode_request, encode_response,
};
use brt_can_protocol::{FrameId, FramePayload, FrameRef};

fn standard(address: u8, bytes: &[u8]) -> FrameRef<'_> {
    FrameRef {
        id: FrameId::Standard(u16::from(address)),
        payload: FramePayload::Data(bytes),
    }
}

fn extended(address: u32, bytes: &[u8]) -> FrameRef<'_> {
    FrameRef {
        id: FrameId::Extended(address),
        payload: FramePayload::Data(bytes),
    }
}

#[test]
fn encodes_every_request_to_its_documented_bytes() {
    let address = Address::standard(1);
    let extended_address = Address::extended(0x18ff_f225, 0xa6).unwrap();
    let cases = [
        (
            Request::ReadPosition,
            FrameId::Standard(1),
            &[4, 1, 1, 0][..],
        ),
        (
            Request::SetStandardAddress(8),
            FrameId::Standard(1),
            &[4, 1, 2, 8],
        ),
        (
            Request::SetExtendedAddress(0x18ff_f225),
            FrameId::Standard(1),
            &[7, 1, 0x22, 0x25, 0xf2, 0xff, 0x18],
        ),
        (
            Request::SetBaudRate(BaudRate::Mbps1),
            FrameId::Standard(1),
            &[4, 1, 3, 1],
        ),
        (
            Request::SetReportingMode(ReportingMode::StandardPosition),
            FrameId::Standard(1),
            &[4, 1, 4, 0xaa],
        ),
        (
            Request::SetReportingPeriod(1000),
            FrameId::Standard(1),
            &[5, 1, 5, 0xe8, 3],
        ),
        (Request::SetZero, FrameId::Standard(1), &[4, 1, 6, 0]),
        (
            Request::SetDirection(Direction::Counterclockwise),
            FrameId::Standard(1),
            &[4, 1, 7, 1],
        ),
        (Request::ReadSpeed, FrameId::Standard(1), &[4, 1, 0x0a, 0]),
        (
            Request::SetSpeedSampleTime(1000),
            FrameId::Standard(1),
            &[5, 1, 0x0b, 0xe8, 3],
        ),
        (Request::SetMidpoint, FrameId::Standard(1), &[4, 1, 0x0c, 1]),
        (
            Request::SetPosition(0x0001_2345),
            FrameId::Standard(1),
            &[7, 1, 0x0d, 0x45, 0x23, 1, 0],
        ),
        (
            Request::SetFiveTurns,
            FrameId::Standard(1),
            &[4, 1, 0x0f, 1],
        ),
        (
            Request::ReadPosition,
            FrameId::Extended(0x18ff_f225),
            &[4, 0xa6, 1, 0],
        ),
    ];

    for (request, id, bytes) in cases {
        let encoded = encode_request(
            if id == FrameId::Extended(0x18ff_f225) {
                extended_address
            } else {
                address
            },
            request,
        )
        .unwrap();
        assert_eq!(encoded.id(), id);
        assert_eq!(encoded.data(), bytes);
        assert_eq!(encoded.dlc(), bytes.len() as u8);
    }
}

#[test]
fn decodes_every_request_from_its_documented_bytes() {
    let address = Address::standard(1);
    let cases = [
        (&[4, 1, 1, 0][..], Request::ReadPosition),
        (&[4, 1, 2, 8], Request::SetStandardAddress(8)),
        (
            &[7, 1, 0x22, 0x25, 0xf2, 0xff, 0x18],
            Request::SetExtendedAddress(0x18ff_f225),
        ),
        (&[4, 1, 3, 0], Request::SetBaudRate(BaudRate::Kbps500)),
        (
            &[4, 1, 4, 0x12],
            Request::SetReportingMode(ReportingMode::ExtendedSignedSpeed),
        ),
        (&[5, 1, 5, 50, 0], Request::SetReportingPeriod(50)),
        (&[4, 1, 6, 0], Request::SetZero),
        (&[4, 1, 7, 0], Request::SetDirection(Direction::Clockwise)),
        (&[4, 1, 0x0a, 0], Request::ReadSpeed),
        (&[5, 1, 0x0b, 0, 0], Request::SetSpeedSampleTime(0)),
        (&[4, 1, 0x0c, 1], Request::SetMidpoint),
        (
            &[7, 1, 0x0d, 0x45, 0x23, 1, 0],
            Request::SetPosition(0x0001_2345),
        ),
        (&[4, 1, 0x0f, 1], Request::SetFiveTurns),
    ];

    for (bytes, expected) in cases {
        assert_eq!(
            decode_request(address, standard(1, bytes)).unwrap(),
            Some(expected)
        );
    }
}

#[test]
fn encodes_and_decodes_responses_without_losing_status_bits() {
    let address = Address::standard(1);
    let cases = [
        (
            Response::Position(0x8001_2345),
            &[7, 1, 1, 0x45, 0x23, 1, 0x80][..],
        ),
        (
            Response::Speed(-74565),
            &[7, 1, 0x0a, 0xbb, 0xdc, 0xfe, 0xff],
        ),
        (
            Response::Ack {
                command: AckCommand::SetReportingMode,
                status: Status(0xf1),
            },
            &[4, 1, 4, 0xf1],
        ),
        (
            Response::SetExtendedAddress(Status32(0xfedc_ba98)),
            &[7, 1, 0x22, 0x98, 0xba, 0xdc, 0xfe],
        ),
    ];
    for (response, bytes) in cases {
        let encoded = encode_response(address, response).unwrap();
        assert_eq!(encoded.id(), FrameId::Standard(1));
        assert_eq!(encoded.data(), bytes);
        assert_eq!(
            decode_response(address, standard(1, bytes)).unwrap(),
            Some(response)
        );
    }
}

#[test]
fn preserves_protocol_value_boundaries_without_physical_range_inference() {
    for address in [Address::standard(0), Address::standard(255)] {
        let encoded = encode_request(address, Request::ReadPosition).unwrap();
        assert_eq!(encoded.id(), address.id());
        assert_eq!(encoded.data(), &[4, address.device_id(), 1, 0]);
        assert_eq!(
            decode_request(address, encoded.as_ref()).unwrap(),
            Some(Request::ReadPosition)
        );
    }
    for address in [
        Address::extended(0, 0).unwrap(),
        Address::extended(0x1fff_ffff, 255).unwrap(),
    ] {
        let encoded = encode_request(address, Request::ReadPosition).unwrap();
        assert_eq!(encoded.id(), address.id());
        assert_eq!(encoded.data(), &[4, address.device_id(), 1, 0]);
        assert_eq!(
            decode_request(address, encoded.as_ref()).unwrap(),
            Some(Request::ReadPosition)
        );
    }
    let address = Address::standard(1);
    for (request, bytes) in [
        (Request::SetStandardAddress(1), &[4, 1, 2, 1][..]),
        (Request::SetStandardAddress(255), &[4, 1, 2, 255]),
        (Request::SetExtendedAddress(0), &[7, 1, 0x22, 0, 0, 0, 0]),
        (
            Request::SetExtendedAddress(0x1fff_ffff),
            &[7, 1, 0x22, 0xff, 0xff, 0xff, 0x1f],
        ),
        (Request::SetReportingPeriod(50), &[5, 1, 5, 50, 0]),
        (
            Request::SetReportingPeriod(u16::MAX),
            &[5, 1, 5, 0xff, 0xff],
        ),
        (Request::SetSpeedSampleTime(0), &[5, 1, 0x0b, 0, 0]),
        (
            Request::SetSpeedSampleTime(u16::MAX),
            &[5, 1, 0x0b, 0xff, 0xff],
        ),
        (Request::SetPosition(0), &[7, 1, 0x0d, 0, 0, 0, 0]),
        (
            Request::SetPosition(u32::MAX),
            &[7, 1, 0x0d, 0xff, 0xff, 0xff, 0xff],
        ),
    ] {
        assert_eq!(encode_request(address, request).unwrap().data(), bytes);
        assert_eq!(
            decode_request(address, standard(1, bytes)).unwrap(),
            Some(request)
        );
    }
    for (response, bytes) in [
        (Response::Position(0), &[7, 1, 1, 0, 0, 0, 0][..]),
        (
            Response::Position(u32::MAX),
            &[7, 1, 1, 0xff, 0xff, 0xff, 0xff],
        ),
        (Response::Speed(0), &[7, 1, 0x0a, 0, 0, 0, 0]),
        (Response::Speed(-1), &[7, 1, 0x0a, 0xff, 0xff, 0xff, 0xff]),
        (Response::Speed(i32::MIN), &[7, 1, 0x0a, 0, 0, 0, 0x80]),
        (
            Response::Speed(i32::MAX),
            &[7, 1, 0x0a, 0xff, 0xff, 0xff, 0x7f],
        ),
        (
            Response::SetExtendedAddress(Status32(u32::MAX)),
            &[7, 1, 0x22, 0xff, 0xff, 0xff, 0xff],
        ),
    ] {
        assert_eq!(encode_response(address, response).unwrap().data(), bytes);
        assert_eq!(
            decode_response(address, standard(1, bytes)).unwrap(),
            Some(response)
        );
    }
}

#[test]
fn uses_an_explicit_extended_address_for_a_response_even_after_address_change() {
    let new_address = Address::extended(0x18ff_f225, 0xa6).unwrap();
    let response = Response::SetExtendedAddress(Status32(u32::MAX));
    let encoded = encode_response(new_address, response).unwrap();
    assert_eq!(encoded.id(), FrameId::Extended(0x18ff_f225));
    assert_eq!(encoded.data(), &[7, 0xa6, 0x22, 0xff, 0xff, 0xff, 0xff]);
    assert_eq!(
        decode_response(new_address, extended(0x18ff_f225, encoded.data())).unwrap(),
        Some(response)
    );
}

#[test]
fn preserves_can_id_and_device_id_as_independent_extended_address_fields() {
    let address = Address::extended(0x18ff_f225, 0xa6).unwrap();
    assert_eq!(address.id(), FrameId::Extended(0x18ff_f225));
    assert_eq!(address.device_id(), 0xa6);
    assert_eq!(
        decode_response(address, extended(0x18ff_f225, &[7, 0x25, 0x22, 0, 0, 0, 0])),
        Err(DecodeError::DeviceIdMismatch {
            expected: 0xa6,
            actual: 0x25,
        })
    );
}

#[test]
fn encodes_and_decodes_every_baud_rate_and_reporting_mode() {
    let address = Address::standard(1);
    let baud_rates = [
        (BaudRate::Kbps500, 0),
        (BaudRate::Mbps1, 1),
        (BaudRate::Kbps250, 2),
        (BaudRate::Kbps125, 3),
        (BaudRate::Kbps100, 4),
    ];
    for (value, byte) in baud_rates {
        let bytes = [4, 1, 3, byte];
        assert_eq!(
            encode_request(address, Request::SetBaudRate(value))
                .unwrap()
                .data(),
            &bytes
        );
        assert_eq!(
            decode_request(address, standard(1, &bytes)).unwrap(),
            Some(Request::SetBaudRate(value))
        );
    }
    let reporting_modes = [
        (ReportingMode::Query, 0),
        (ReportingMode::StandardPosition, 0xaa),
        (ReportingMode::StandardSignedSpeed, 2),
        (ReportingMode::StandardUnsignedSpeed, 7),
        (ReportingMode::ExtendedPosition, 0x18),
        (ReportingMode::ExtendedSignedSpeed, 0x12),
        (ReportingMode::ExtendedUnsignedSpeed, 0x17),
    ];
    for (value, byte) in reporting_modes {
        let bytes = [4, 1, 4, byte];
        assert_eq!(
            encode_request(address, Request::SetReportingMode(value))
                .unwrap()
                .data(),
            &bytes
        );
        assert_eq!(
            decode_request(address, standard(1, &bytes)).unwrap(),
            Some(Request::SetReportingMode(value))
        );
    }
}

#[test]
fn encodes_and_decodes_an_ack_for_every_ordinary_write_function() {
    let address = Address::standard(1);
    let commands = [
        (AckCommand::SetStandardAddress, 2),
        (AckCommand::SetBaudRate, 3),
        (AckCommand::SetReportingMode, 4),
        (AckCommand::SetReportingPeriod, 5),
        (AckCommand::SetZero, 6),
        (AckCommand::SetDirection, 7),
        (AckCommand::SetSpeedSampleTime, 0x0b),
        (AckCommand::SetMidpoint, 0x0c),
        (AckCommand::SetPosition, 0x0d),
        (AckCommand::SetFiveTurns, 0x0f),
    ];
    for (command, function) in commands {
        let response = Response::Ack {
            command,
            status: Status(0xff),
        };
        let bytes = [4, 1, function, 0xff];
        assert_eq!(encode_response(address, response).unwrap().data(), &bytes);
        assert_eq!(
            decode_response(address, standard(1, &bytes)).unwrap(),
            Some(response)
        );
    }
}

#[test]
fn validates_requests_and_address_construction_at_the_protocol_boundary() {
    assert_eq!(
        encode_request(Address::standard(0), Request::SetStandardAddress(0)),
        Err(EncodeError::InvalidStandardAddress(0))
    );
    assert_eq!(
        encode_request(
            Address::standard(1),
            Request::SetExtendedAddress(0x2000_0000)
        ),
        Err(EncodeError::InvalidExtendedCanId(0x2000_0000))
    );
    assert_eq!(
        encode_request(Address::standard(1), Request::SetReportingPeriod(49)),
        Err(EncodeError::InvalidReportingPeriod(49))
    );
    assert_eq!(
        encode_request(Address::standard(1), Request::SetReportingPeriod(0)),
        Err(EncodeError::InvalidReportingPeriod(0))
    );
    assert_eq!(
        Address::extended(0x2000_0000, 0),
        Err(EncodeError::InvalidExtendedCanId(0x2000_0000))
    );
    assert_eq!(Address::standard(0).device_id(), 0);
}

#[test]
fn rejects_malformed_matched_frames_and_never_accepts_padding() {
    let address = Address::standard(1);
    assert_eq!(
        decode_request(address, standard(1, &[4, 1, 0x0a, 1])),
        Err(DecodeError::InvalidFixedParameter {
            function: 0x0a,
            expected: 0,
            actual: 1,
        })
    );
    assert_eq!(
        decode_request(address, standard(1, &[4, 1, 3, 5])),
        Err(DecodeError::InvalidBaudRate(5))
    );
    assert_eq!(
        decode_request(address, standard(1, &[4, 1, 1])),
        Err(DecodeError::LengthMismatch {
            declared: 4,
            actual: 3,
        })
    );
    assert_eq!(
        decode_request(address, standard(1, &[4, 1, 1, 0, 0, 0, 0, 0])),
        Err(DecodeError::LengthMismatch {
            declared: 4,
            actual: 8,
        })
    );
    assert_eq!(
        decode_request(address, standard(1, &[4, 2, 1, 0])),
        Err(DecodeError::DeviceIdMismatch {
            expected: 1,
            actual: 2,
        })
    );
    assert_eq!(
        decode_request(address, standard(1, &[4, 1, 1, 1])),
        Err(DecodeError::InvalidFixedParameter {
            function: 1,
            expected: 0,
            actual: 1,
        })
    );
    assert_eq!(
        decode_request(address, standard(1, &[4, 1, 7, 2])),
        Err(DecodeError::InvalidDirection(2))
    );
    assert_eq!(
        decode_request(address, standard(1, &[4, 1, 3, 5])),
        Err(DecodeError::InvalidBaudRate(5))
    );
    assert_eq!(
        decode_request(address, standard(1, &[4, 1, 3, 255])),
        Err(DecodeError::InvalidBaudRate(255))
    );
    assert_eq!(
        decode_request(address, standard(1, &[4, 1, 4, 0xff])),
        Err(DecodeError::InvalidReportingMode(0xff))
    );
    assert_eq!(
        decode_request(address, standard(1, &[5, 1, 5, 49, 0])),
        Err(DecodeError::InvalidReportingPeriod(49))
    );
    assert_eq!(
        decode_request(address, standard(1, &[4, 1, 2, 0])),
        Err(DecodeError::InvalidStandardAddress(0))
    );
    assert_eq!(
        decode_request(address, standard(1, &[4, 1, 6, 1])),
        Err(DecodeError::InvalidFixedParameter {
            function: 6,
            expected: 0,
            actual: 1,
        })
    );
    assert_eq!(
        decode_request(address, standard(1, &[4, 1, 0x0c, 0])),
        Err(DecodeError::InvalidFixedParameter {
            function: 0x0c,
            expected: 1,
            actual: 0,
        })
    );
    assert_eq!(
        decode_request(address, standard(1, &[4, 1, 0x0f, 0])),
        Err(DecodeError::InvalidFixedParameter {
            function: 0x0f,
            expected: 1,
            actual: 0,
        })
    );
    assert_eq!(
        decode_response(address, standard(1, &[4, 1, 0x22, 0, 0, 0, 0])),
        Err(DecodeError::LengthMismatch {
            declared: 4,
            actual: 7,
        })
    );
    assert_eq!(
        decode_request(address, standard(1, &[7, 1, 4, 0, 0, 0, 0])),
        Err(DecodeError::InvalidLength {
            function: 4,
            expected: 4,
            actual: 7,
        })
    );
    assert_eq!(
        decode_request(address, standard(1, &[9, 1, 1, 0, 0, 0, 0, 0, 0])),
        Err(DecodeError::DataLengthExceedsClassicCan(9))
    );
    assert_eq!(
        decode_request(address, standard(1, &[9, 1, 1, 0, 0, 0, 0, 0])),
        Err(DecodeError::DeclaredLengthExceedsClassicCan(9))
    );
}

#[test]
fn rejects_remote_and_fd_even_when_the_identifier_is_unrelated() {
    let address = Address::standard(1);
    assert_eq!(
        decode_request(
            address,
            FrameRef {
                id: FrameId::Standard(2),
                payload: FramePayload::Remote { dlc: 0 },
            },
        ),
        Err(DecodeError::UnsupportedRemoteFrame)
    );
    assert_eq!(
        decode_response(
            address,
            FrameRef {
                id: FrameId::Extended(2),
                payload: FramePayload::Fd(&[]),
            },
        ),
        Err(DecodeError::UnsupportedCanFd)
    );
    assert_eq!(
        decode_request(
            address,
            FrameRef {
                id: FrameId::Standard(1),
                payload: FramePayload::Remote { dlc: 4 },
            },
        ),
        Err(DecodeError::UnsupportedRemoteFrame)
    );
    assert_eq!(
        decode_response(
            address,
            FrameRef {
                id: FrameId::Standard(1),
                payload: FramePayload::Fd(&[7, 1, 1, 0, 0, 0, 0]),
            },
        ),
        Err(DecodeError::UnsupportedCanFd)
    );
}

#[test]
fn ignores_legal_other_identifiers_and_unknown_functions_without_guessing_a_speed_format() {
    let address = Address::standard(1);
    assert_eq!(decode_request(address, standard(2, &[])).unwrap(), None);
    assert_eq!(
        decode_response(
            address,
            FrameRef {
                id: FrameId::Standard(0x100),
                payload: FramePayload::Data(&[]),
            },
        )
        .unwrap(),
        None
    );
    assert_eq!(
        decode_response(address, standard(1, &[4, 1, 0x7e, 0])).unwrap(),
        None
    );
}

#[test]
fn validates_can_identifier_ranges_before_protocol_classification() {
    let address = Address::standard(1);
    assert_eq!(
        decode_request(
            address,
            FrameRef {
                id: FrameId::Standard(0x800),
                payload: FramePayload::Data(&[]),
            },
        ),
        Err(DecodeError::InvalidStandardCanId(0x800))
    );
    assert_eq!(
        decode_response(
            address,
            FrameRef {
                id: FrameId::Extended(0x2000_0000),
                payload: FramePayload::Data(&[]),
            },
        ),
        Err(DecodeError::InvalidExtendedCanId(0x2000_0000))
    );
    assert_eq!(
        decode_request(address, standard(1, &[7, 1, 0x22, 0xff, 0xff, 0xff, 0xff])),
        Err(DecodeError::InvalidExtendedCanId(u32::MAX))
    );
}

#[test]
fn rejects_wrong_protocol_lengths_for_each_known_request_function() {
    let address = Address::standard(1);
    let cases = [
        (0x01, 4, &[5, 1, 0x01, 0, 0][..]),
        (0x02, 4, &[5, 1, 0x02, 1, 0]),
        (0x22, 7, &[4, 1, 0x22, 0]),
        (0x03, 4, &[5, 1, 0x03, 0, 0]),
        (0x04, 4, &[5, 1, 0x04, 0, 0]),
        (0x05, 5, &[4, 1, 0x05, 50]),
        (0x06, 4, &[5, 1, 0x06, 0, 0]),
        (0x07, 4, &[5, 1, 0x07, 0, 0]),
        (0x0a, 4, &[5, 1, 0x0a, 0, 0]),
        (0x0b, 5, &[4, 1, 0x0b, 0]),
        (0x0c, 4, &[5, 1, 0x0c, 1, 0]),
        (0x0d, 7, &[4, 1, 0x0d, 0]),
        (0x0f, 4, &[5, 1, 0x0f, 1, 0]),
    ];
    for (function, expected, bytes) in cases {
        assert_eq!(
            decode_request(address, standard(1, bytes)),
            Err(DecodeError::InvalidLength {
                function,
                expected,
                actual: bytes.len() as u8,
            })
        );
    }
}

#[test]
fn does_not_conflate_address_formats_or_current_and_new_standard_addresses() {
    let old = Address::standard(1);
    let new = Address::standard(8);
    let acknowledgement = encode_response(
        new,
        Response::Ack {
            command: AckCommand::SetStandardAddress,
            status: Status(0),
        },
    )
    .unwrap();
    assert_eq!(
        encode_request(old, Request::SetStandardAddress(8))
            .unwrap()
            .id(),
        FrameId::Standard(1)
    );
    assert_eq!(acknowledgement.id(), FrameId::Standard(8));
    assert_eq!(acknowledgement.data(), &[4, 8, 2, 0]);
    assert_eq!(
        decode_response(old, acknowledgement.as_ref()).unwrap(),
        None
    );
    assert_eq!(
        decode_response(new, acknowledgement.as_ref()).unwrap(),
        Some(Response::Ack {
            command: AckCommand::SetStandardAddress,
            status: Status(0),
        })
    );
    let standard = Address::standard(1);
    let extended = Address::extended(1, 1).unwrap();
    let standard_frame = encode_request(standard, Request::ReadPosition).unwrap();
    let extended_frame = encode_request(extended, Request::ReadPosition).unwrap();
    assert_eq!(
        decode_request(extended, standard_frame.as_ref()).unwrap(),
        None
    );
    assert_eq!(
        decode_request(standard, extended_frame.as_ref()).unwrap(),
        None
    );
}

#[test]
fn rejects_truncated_or_extra_data_for_every_response_function() {
    // 独立列出手册的响应长度，不从编码器取得期望长度。
    let functions = [
        (0x01, 7),
        (0x02, 4),
        (0x22, 7),
        (0x03, 4),
        (0x04, 4),
        (0x05, 4),
        (0x06, 4),
        (0x07, 4),
        (0x0a, 7),
        (0x0b, 4),
        (0x0c, 4),
        (0x0d, 4),
        (0x0f, 4),
    ];
    for (function, expected) in functions {
        for length in 0..=8u8 {
            if length == expected {
                continue;
            }
            let bytes = [length, 1, function, 0, 0, 0, 0, 0];
            let error = if length < 3 {
                DecodeError::IncompleteHeader {
                    actual: usize::from(length),
                }
            } else {
                DecodeError::InvalidLength {
                    function,
                    expected,
                    actual: length,
                }
            };
            assert_eq!(
                decode_response(
                    Address::standard(1),
                    standard(1, &bytes[..usize::from(length)])
                ),
                Err(error),
            );
        }
    }
}

#[test]
fn validates_unknown_function_headers_before_returning_unrelated() {
    let address = Address::standard(1);
    assert_eq!(
        decode_response(address, standard(1, &[4, 2, 0x7e, 0])),
        Err(DecodeError::DeviceIdMismatch {
            expected: 1,
            actual: 2
        }),
    );
    assert_eq!(
        decode_response(address, standard(1, &[7, 1, 0x7e, 0])),
        Err(DecodeError::LengthMismatch {
            declared: 7,
            actual: 4
        }),
    );
    assert_eq!(
        decode_request(address, standard(1, &[3, 1, 0x7e])),
        Ok(None)
    );
}
