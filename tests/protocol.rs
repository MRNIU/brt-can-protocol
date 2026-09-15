// Copyright The brt-can-protocol Contributors
// 本文件以独立给定字节验证 BRT CAN 协议编解码与拒绝规则。

//! BRT CAN 协议的给定字节测试。

use brt_can_protocol::protocol::{
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

fn request_case(address: Address, request: Request, bytes: &[u8]) {
    let encoded = encode_request(address, request).unwrap();
    assert_eq!(encoded.id(), address.id());
    assert_eq!(encoded.data(), bytes);
    assert_eq!(encoded.dlc(), bytes.len() as u8);
    let input = match address.id() {
        FrameId::Standard(id) => standard(id as u8, bytes),
        FrameId::Extended(id) => extended(id, bytes),
    };
    assert_eq!(decode_request(address, input).unwrap(), Some(request));
}

fn response_case(address: Address, response: Response, bytes: &[u8]) {
    let encoded = encode_response(address, response).unwrap();
    assert_eq!(encoded.id(), address.id());
    assert_eq!(encoded.data(), bytes);
    assert_eq!(encoded.dlc(), bytes.len() as u8);
    let input = match address.id() {
        FrameId::Standard(id) => standard(id as u8, bytes),
        FrameId::Extended(id) => extended(id, bytes),
    };
    assert_eq!(decode_response(address, input).unwrap(), Some(response));
}

#[test]
fn requests_encode_and_decode_every_documented_function() {
    let address = Address::standard(1);
    // 0x22、0x0B、0x0D 的请求参数使用大端；
    // 0x22 编码统一采用 LEN=7，0x05 的回传周期仍为小端。
    for (request, bytes) in [
        (Request::ReadPosition, &[4, 1, 1, 0][..]),
        (Request::SetStandardAddress(8), &[4, 1, 2, 8]),
        (
            Request::SetExtendedAddress(0x18ff_f225),
            &[7, 1, 0x22, 0x18, 0xff, 0xf2, 0x25],
        ),
        (Request::SetBaudRate(BaudRate::Mbps1), &[4, 1, 3, 1]),
        (
            Request::SetReportingMode(ReportingMode::StandardPosition),
            &[4, 1, 4, 0xaa],
        ),
        (Request::SetReportingPeriod(1000), &[5, 1, 5, 0xe8, 3]),
        (Request::SetZero, &[4, 1, 6, 0]),
        (
            Request::SetDirection(Direction::Counterclockwise),
            &[4, 1, 7, 1],
        ),
        (Request::ReadSpeed, &[4, 1, 0x0a, 0]),
        (Request::SetSpeedSampleTime(1000), &[5, 1, 0x0b, 3, 0xe8]),
        (Request::SetMidpoint, &[4, 1, 0x0c, 1]),
        (
            Request::SetPosition(0x0001_2345),
            &[7, 1, 0x0d, 0, 1, 0x23, 0x45],
        ),
        (Request::SetFiveTurns, &[4, 1, 0x0f, 1]),
    ] {
        request_case(address, request, bytes);
    }
    request_case(
        Address::extended(0x18ff_f225, 0xa6).unwrap(),
        Request::ReadPosition,
        &[4, 0xa6, 1, 0],
    );
}

#[test]
fn big_endian_requests_decode_given_bytes() {
    for address in [
        Address::standard(1),
        Address::extended(0x18ff_f201, 1).unwrap(),
    ] {
        for (bytes, request) in [
            (
                &[7, 1, 0x22, 0x18, 0xff, 0xf2, 0x25][..],
                Request::SetExtendedAddress(0x18ff_f225),
            ),
            (&[5, 1, 0x0b, 3, 0xe8], Request::SetSpeedSampleTime(1000)),
            (&[7, 1, 0x0d, 0, 1, 0x23, 0x45], Request::SetPosition(74565)),
            (
                &[7, 1, 0x0d, 0x89, 0xab, 0xcd, 0xef],
                Request::SetPosition(0x89ab_cdef),
            ),
        ] {
            let frame = FrameRef {
                id: address.id(),
                payload: FramePayload::Data(bytes),
            };
            assert_eq!(decode_request(address, frame), Ok(Some(request)));
            assert_eq!(encode_request(address, request).unwrap().data(), bytes);
        }
    }
}

#[test]
fn extended_address_request_accepts_len_four_only_with_seven_data_bytes() {
    for address in [
        Address::standard(1),
        Address::extended(0x18ff_f201, 1).unwrap(),
    ] {
        let frame = FrameRef {
            id: address.id(),
            payload: FramePayload::Data(&[4, 1, 0x22, 0x18, 0xff, 0xf2, 0x25]),
        };
        assert_eq!(
            decode_request(address, frame),
            Ok(Some(Request::SetExtendedAddress(0x18ff_f225)))
        );
        assert_eq!(
            decode_response(address, frame),
            Err(DecodeError::LengthMismatch {
                declared: 4,
                actual: 7,
            })
        );
    }
}

#[test]
fn extended_address_length_exception_keeps_header_and_frame_validation() {
    let address = Address::standard(1);
    for (bytes, error) in [
        (
            &[4, 1, 0x22, 0x18, 0xff, 0xf2][..],
            DecodeError::LengthMismatch {
                declared: 4,
                actual: 6,
            },
        ),
        (
            &[4, 1, 0x22, 0x18, 0xff, 0xf2, 0x25, 0],
            DecodeError::LengthMismatch {
                declared: 4,
                actual: 8,
            },
        ),
        (
            &[5, 1, 0x22, 0x18, 0xff, 0xf2, 0x25],
            DecodeError::LengthMismatch {
                declared: 5,
                actual: 7,
            },
        ),
        (
            &[4, 1, 0x0d, 0, 1, 0x23, 0x45],
            DecodeError::LengthMismatch {
                declared: 4,
                actual: 7,
            },
        ),
        (
            &[4, 1, 0x0b, 3, 0xe8],
            DecodeError::LengthMismatch {
                declared: 4,
                actual: 5,
            },
        ),
        (
            &[4, 1, 0x7e, 0, 0, 0, 0],
            DecodeError::LengthMismatch {
                declared: 4,
                actual: 7,
            },
        ),
        (
            &[4, 2, 0x22, 0x18, 0xff, 0xf2, 0x25],
            DecodeError::DeviceIdMismatch {
                expected: 1,
                actual: 2,
            },
        ),
        (
            &[4, 1, 0x22, 0x20, 0, 0, 0],
            DecodeError::InvalidExtendedCanId(0x2000_0000),
        ),
        (
            &[7, 1, 0x22, 0x20, 0, 0, 0],
            DecodeError::InvalidExtendedCanId(0x2000_0000),
        ),
    ] {
        assert_eq!(decode_request(address, standard(1, bytes)), Err(error));
    }
    for (payload, error) in [
        (
            FramePayload::Remote { dlc: 7 },
            DecodeError::UnsupportedRemoteFrame,
        ),
        (
            FramePayload::Fd(&[4, 1, 0x22, 0x18, 0xff, 0xf2, 0x25]),
            DecodeError::UnsupportedCanFd,
        ),
    ] {
        assert_eq!(
            decode_request(
                address,
                FrameRef {
                    id: address.id(),
                    payload
                }
            ),
            Err(error)
        );
    }
}

#[test]
fn extended_address_example_response_matches_explicit_new_address() {
    let old = Address::standard(1);
    let new = Address::extended(0x18ff_f225, 0x25).unwrap();
    let bytes = &[7, 0x25, 0x22, 0, 0, 0, 0];
    response_case(new, Response::SetExtendedAddress(Status32(0)), bytes);
    assert_eq!(decode_response(old, extended(0x18ff_f225, bytes)), Ok(None));
}

#[test]
fn extended_address_response_decodes_big_endian_status_without_losing_bits() {
    // 非零且各不相同的字节用于区分端序并验证完整状态位。
    let address = Address::extended(0x18ff_f225, 0x25).unwrap();
    assert_eq!(
        decode_response(
            address,
            extended(0x18ff_f225, &[7, 0x25, 0x22, 0xfe, 0xdc, 0xba, 0x98]),
        ),
        Ok(Some(Response::SetExtendedAddress(Status32(0xfedc_ba98))))
    );
}

#[test]
fn baud_rates_and_reporting_modes_keep_all_wire_values() {
    let address = Address::standard(1);
    for (value, byte) in [
        (BaudRate::Kbps500, 0),
        (BaudRate::Mbps1, 1),
        (BaudRate::Kbps250, 2),
        (BaudRate::Kbps125, 3),
        (BaudRate::Kbps100, 4),
    ] {
        request_case(address, Request::SetBaudRate(value), &[4, 1, 3, byte]);
    }
    for (value, byte) in [
        (ReportingMode::Query, 0),
        (ReportingMode::StandardPosition, 0xaa),
        (ReportingMode::StandardSignedSpeed, 2),
        (ReportingMode::StandardUnsignedSpeed, 7),
        (ReportingMode::ExtendedPosition, 0x18),
        (ReportingMode::ExtendedSignedSpeed, 0x12),
        (ReportingMode::ExtendedUnsignedSpeed, 0x17),
    ] {
        request_case(address, Request::SetReportingMode(value), &[4, 1, 4, byte]);
    }
}

#[test]
fn responses_encode_and_decode_every_documented_function_and_status_bit() {
    let address = Address::standard(1);
    for (command, function) in [
        (AckCommand::SetStandardAddress, 0x02),
        (AckCommand::SetBaudRate, 0x03),
        (AckCommand::SetReportingMode, 0x04),
        (AckCommand::SetReportingPeriod, 0x05),
        (AckCommand::SetZero, 0x06),
        (AckCommand::SetDirection, 0x07),
        (AckCommand::SetSpeedSampleTime, 0x0b),
        (AckCommand::SetMidpoint, 0x0c),
        (AckCommand::SetPosition, 0x0d),
        (AckCommand::SetFiveTurns, 0x0f),
    ] {
        response_case(
            address,
            Response::Ack {
                command,
                status: Status(0xff),
            },
            &[4, 1, function, 0xff],
        );
    }
    for (response, bytes) in [
        (
            Response::Position(0x8001_2345),
            &[7, 1, 1, 0x45, 0x23, 1, 0x80][..],
        ),
        (
            Response::Speed(-74565),
            &[7, 1, 0x0a, 0xbb, 0xdc, 0xfe, 0xff],
        ),
        (
            Response::SetExtendedAddress(Status32(0xfedc_ba98)),
            &[7, 1, 0x22, 0xfe, 0xdc, 0xba, 0x98],
        ),
    ] {
        response_case(address, response, bytes);
    }
}

#[test]
fn protocol_value_boundaries_are_not_physical_range_inference() {
    for address in [Address::standard(0), Address::standard(255)] {
        request_case(
            address,
            Request::ReadPosition,
            &[4, address.device_id(), 1, 0],
        );
    }
    for address in [
        Address::extended(0, 0).unwrap(),
        Address::extended(0x1fff_ffff, 255).unwrap(),
    ] {
        request_case(
            address,
            Request::ReadPosition,
            &[4, address.device_id(), 1, 0],
        );
    }
    let address = Address::standard(1);
    for (request, bytes) in [
        (Request::SetStandardAddress(1), &[4, 1, 2, 1][..]),
        (Request::SetStandardAddress(255), &[4, 1, 2, 255]),
        (Request::SetDirection(Direction::Clockwise), &[4, 1, 7, 0]),
        (Request::SetExtendedAddress(0), &[7, 1, 0x22, 0, 0, 0, 0]),
        (
            Request::SetExtendedAddress(0x1fff_ffff),
            &[7, 1, 0x22, 0x1f, 0xff, 0xff, 0xff],
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
        request_case(address, request, bytes);
    }
    for (response, bytes) in [
        (Response::Position(0), &[7, 1, 1, 0, 0, 0, 0][..]),
        (
            Response::Position(u32::MAX),
            &[7, 1, 1, 0xff, 0xff, 0xff, 0xff],
        ),
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
        response_case(address, response, bytes);
    }
}

#[test]
fn addresses_keep_identifier_format_device_id_and_new_address_independent() {
    let extended_address = Address::extended(0x18ff_f225, 0xa6).unwrap();
    assert_eq!(extended_address.id(), FrameId::Extended(0x18ff_f225));
    assert_eq!(extended_address.device_id(), 0xa6);
    assert_eq!(
        decode_response(
            extended_address,
            extended(0x18ff_f225, &[7, 0x25, 0x22, 0, 0, 0, 0])
        ),
        Err(DecodeError::DeviceIdMismatch {
            expected: 0xa6,
            actual: 0x25
        })
    );
    response_case(
        extended_address,
        Response::SetExtendedAddress(Status32(u32::MAX)),
        &[7, 0xa6, 0x22, 0xff, 0xff, 0xff, 0xff],
    );
    let old = Address::standard(1);
    let new = Address::standard(8);
    request_case(old, Request::SetStandardAddress(8), &[4, 1, 2, 8]);
    let acknowledgement = Response::Ack {
        command: AckCommand::SetStandardAddress,
        status: Status(0),
    };
    response_case(new, acknowledgement, &[4, 8, 2, 0]);
    assert_eq!(
        decode_response(old, standard(8, &[4, 8, 2, 0])).unwrap(),
        None
    );
    let standard_frame = encode_request(old, Request::ReadPosition).unwrap();
    let extended = Address::extended(1, 1).unwrap();
    assert_eq!(
        decode_request(extended, standard_frame.as_ref()).unwrap(),
        None
    );
    assert_eq!(
        decode_request(
            old,
            encode_request(extended, Request::ReadPosition)
                .unwrap()
                .as_ref()
        )
        .unwrap(),
        None
    );
}

#[test]
fn rejects_invalid_values_and_malformed_matched_frames() {
    let address = Address::standard(1);
    assert_eq!(
        encode_request(Address::standard(0), Request::SetStandardAddress(0)),
        Err(EncodeError::InvalidStandardAddress(0))
    );
    assert_eq!(
        encode_request(address, Request::SetExtendedAddress(0x2000_0000)),
        Err(EncodeError::InvalidExtendedCanId(0x2000_0000))
    );
    assert_eq!(
        encode_request(address, Request::SetReportingPeriod(49)),
        Err(EncodeError::InvalidReportingPeriod(49))
    );
    assert_eq!(
        Address::extended(0x2000_0000, 0),
        Err(EncodeError::InvalidExtendedCanId(0x2000_0000))
    );
    for (bytes, error) in [
        (
            &[4, 1, 1, 1][..],
            DecodeError::InvalidFixedParameter {
                function: 1,
                expected: 0,
                actual: 1,
            },
        ),
        (&[4, 1, 2, 0], DecodeError::InvalidStandardAddress(0)),
        (&[4, 1, 3, 5], DecodeError::InvalidBaudRate(5)),
        (&[4, 1, 4, 0xff], DecodeError::InvalidReportingMode(0xff)),
        (&[5, 1, 5, 49, 0], DecodeError::InvalidReportingPeriod(49)),
        (
            &[4, 1, 6, 1],
            DecodeError::InvalidFixedParameter {
                function: 6,
                expected: 0,
                actual: 1,
            },
        ),
        (&[4, 1, 7, 2], DecodeError::InvalidDirection(2)),
        (
            &[4, 1, 0x0a, 1],
            DecodeError::InvalidFixedParameter {
                function: 0x0a,
                expected: 0,
                actual: 1,
            },
        ),
        (
            &[4, 1, 0x0c, 0],
            DecodeError::InvalidFixedParameter {
                function: 0x0c,
                expected: 1,
                actual: 0,
            },
        ),
        (
            &[4, 1, 0x0f, 0],
            DecodeError::InvalidFixedParameter {
                function: 0x0f,
                expected: 1,
                actual: 0,
            },
        ),
        (
            &[4, 2, 1, 0],
            DecodeError::DeviceIdMismatch {
                expected: 1,
                actual: 2,
            },
        ),
        (
            &[4, 1, 1],
            DecodeError::LengthMismatch {
                declared: 4,
                actual: 3,
            },
        ),
        (
            &[4, 1, 1, 0, 0, 0, 0, 0],
            DecodeError::LengthMismatch {
                declared: 4,
                actual: 8,
            },
        ),
        (
            &[9, 1, 1, 0, 0, 0, 0, 0, 0],
            DecodeError::DataLengthExceedsClassicCan(9),
        ),
        (
            &[9, 1, 1, 0, 0, 0, 0, 0],
            DecodeError::DeclaredLengthExceedsClassicCan(9),
        ),
    ] {
        assert_eq!(decode_request(address, standard(1, bytes)), Err(error));
    }
}

#[test]
fn rejects_each_known_function_wrong_length_and_response_truncation_or_padding() {
    let address = Address::standard(1);
    // 仅 0x22 请求允许使用 LEN=4；应答必须使用 LEN=7。
    assert_eq!(
        decode_response(address, standard(1, &[4, 1, 0x22, 0, 0, 0, 0])),
        Err(DecodeError::LengthMismatch {
            declared: 4,
            actual: 7
        })
    );
    for (function, expected, wrong) in [
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
    ] {
        assert_eq!(
            decode_request(address, standard(1, wrong)),
            Err(DecodeError::InvalidLength {
                function,
                expected,
                actual: wrong.len() as u8,
            })
        );
    }
    for (function, expected) in [
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
    ] {
        for length in 0..=8u8 {
            if length == expected {
                continue;
            }
            let bytes = [length, 1, function, 0, 0, 0, 0, 0];
            let error = if length < 3 {
                DecodeError::IncompleteHeader {
                    actual: length as usize,
                }
            } else {
                DecodeError::InvalidLength {
                    function,
                    expected,
                    actual: length,
                }
            };
            assert_eq!(
                decode_response(address, standard(1, &bytes[..length as usize])),
                Err(error)
            );
        }
    }
}

#[test]
fn rejects_rtr_and_fd_before_identifier_matching_and_validates_unknown_headers() {
    let address = Address::standard(1);
    for frame in [
        FrameRef {
            id: FrameId::Standard(2),
            payload: FramePayload::Remote { dlc: 0 },
        },
        FrameRef {
            id: FrameId::Standard(1),
            payload: FramePayload::Remote { dlc: 4 },
        },
    ] {
        assert_eq!(
            decode_request(address, frame),
            Err(DecodeError::UnsupportedRemoteFrame)
        );
    }
    for frame in [
        FrameRef {
            id: FrameId::Extended(2),
            payload: FramePayload::Fd(&[]),
        },
        FrameRef {
            id: FrameId::Standard(1),
            payload: FramePayload::Fd(&[7, 1, 1, 0, 0, 0, 0]),
        },
    ] {
        assert_eq!(
            decode_response(address, frame),
            Err(DecodeError::UnsupportedCanFd)
        );
    }
    assert_eq!(decode_request(address, standard(2, &[])).unwrap(), None);
    assert_eq!(
        decode_response(address, standard(1, &[4, 1, 0x7e, 0])).unwrap(),
        None
    );
    assert_eq!(
        decode_response(address, standard(1, &[4, 2, 0x7e, 0])),
        Err(DecodeError::DeviceIdMismatch {
            expected: 1,
            actual: 2
        })
    );
    assert_eq!(
        decode_response(address, standard(1, &[7, 1, 0x7e, 0])),
        Err(DecodeError::LengthMismatch {
            declared: 7,
            actual: 4
        })
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
            }
        ),
        Err(DecodeError::InvalidStandardCanId(0x800))
    );
    assert_eq!(
        decode_response(
            address,
            FrameRef {
                id: FrameId::Extended(0x2000_0000),
                payload: FramePayload::Data(&[]),
            }
        ),
        Err(DecodeError::InvalidExtendedCanId(0x2000_0000))
    );
    assert_eq!(
        decode_request(address, standard(1, &[7, 1, 0x22, 0xff, 0xff, 0xff, 0xff])),
        Err(DecodeError::InvalidExtendedCanId(u32::MAX))
    );
}
