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

fn number(text: &str) -> i64 {
    match text.strip_prefix("0x") {
        Some(hex) => i64::from_str_radix(hex, 16).unwrap(),
        None => text.parse().unwrap(),
    }
}

fn fixture_request(function: u8, value: i64) -> Request {
    match function {
        0x01 => Request::ReadPosition,
        0x02 => Request::SetStandardAddress(u8::try_from(value).unwrap()),
        0x22 => Request::SetExtendedAddress(u32::try_from(value).unwrap()),
        0x03 => Request::SetBaudRate(match value {
            0 => BaudRate::Kbps500,
            1 => BaudRate::Mbps1,
            2 => BaudRate::Kbps250,
            3 => BaudRate::Kbps125,
            4 => BaudRate::Kbps100,
            _ => panic!("未知波特率测试值 {value}"),
        }),
        0x04 => Request::SetReportingMode(match value {
            0 => ReportingMode::Query,
            0xaa => ReportingMode::StandardPosition,
            2 => ReportingMode::StandardSignedSpeed,
            7 => ReportingMode::StandardUnsignedSpeed,
            0x18 => ReportingMode::ExtendedPosition,
            0x12 => ReportingMode::ExtendedSignedSpeed,
            0x17 => ReportingMode::ExtendedUnsignedSpeed,
            _ => panic!("未知回传模式测试值 {value}"),
        }),
        0x05 => Request::SetReportingPeriod(u16::try_from(value).unwrap()),
        0x06 => Request::SetZero,
        0x07 => Request::SetDirection(match value {
            0 => Direction::Clockwise,
            1 => Direction::Counterclockwise,
            _ => panic!("未知方向测试值 {value}"),
        }),
        0x0a => Request::ReadSpeed,
        0x0b => Request::SetSpeedSampleTime(u16::try_from(value).unwrap()),
        0x0c => Request::SetMidpoint,
        0x0d => Request::SetPosition(u32::try_from(value).unwrap()),
        0x0f => Request::SetFiveTurns,
        _ => panic!("未知请求功能码 {function:#04x}"),
    }
}

fn fixture_response(function: u8, value: i64) -> Response {
    match function {
        0x01 => Response::Position(u32::try_from(value).unwrap()),
        0x0a => Response::Speed(i32::try_from(value).unwrap()),
        0x22 => Response::SetExtendedAddress(Status32(u32::try_from(value).unwrap())),
        _ => Response::Ack {
            command: match function {
                0x02 => AckCommand::SetStandardAddress,
                0x03 => AckCommand::SetBaudRate,
                0x04 => AckCommand::SetReportingMode,
                0x05 => AckCommand::SetReportingPeriod,
                0x06 => AckCommand::SetZero,
                0x07 => AckCommand::SetDirection,
                0x0b => AckCommand::SetSpeedSampleTime,
                0x0c => AckCommand::SetMidpoint,
                0x0d => AckCommand::SetPosition,
                0x0f => AckCommand::SetFiveTurns,
                _ => panic!("未知应答功能码 {function:#04x}"),
            },
            status: Status(u8::try_from(value).unwrap()),
        },
    }
}

fn decode_error_code(error: DecodeError) -> &'static str {
    match error {
        DecodeError::InvalidStandardCanId(_) => "invalid_standard_can_id",
        DecodeError::InvalidExtendedCanId(_) => "invalid_extended_can_id",
        DecodeError::UnsupportedRemoteFrame => "unsupported_remote_frame",
        DecodeError::UnsupportedCanFd => "unsupported_can_fd",
        DecodeError::DataLengthExceedsClassicCan(_) => "data_length_exceeds_classic_can",
        DecodeError::IncompleteHeader { .. } => "incomplete_header",
        DecodeError::LengthMismatch { .. } => "length_mismatch",
        DecodeError::DeclaredLengthExceedsClassicCan(_) => "declared_length_exceeds_classic_can",
        DecodeError::DeviceIdMismatch { .. } => "device_id_mismatch",
        DecodeError::InvalidLength { .. } => "invalid_length",
        DecodeError::InvalidFixedParameter { .. } => "invalid_fixed_parameter",
        DecodeError::InvalidStandardAddress(_) => "invalid_standard_address",
        DecodeError::InvalidBaudRate(_) => "invalid_baud_rate",
        DecodeError::InvalidReportingMode(_) => "invalid_reporting_mode",
        DecodeError::InvalidDirection(_) => "invalid_direction",
        DecodeError::InvalidReportingPeriod(_) => "invalid_reporting_period",
    }
}

#[test]
fn protocol_matches_shared_vectors() {
    for line in include_str!("fixtures/brt_v2_7.tsv").lines() {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let columns: Vec<_> = line.split('\t').collect();
        let [
            case,
            operation,
            target_format,
            target_id,
            device_id,
            frame_format,
            frame_id,
            frame_kind,
            payload,
            function,
            value,
            result,
        ] = columns.as_slice()
        else {
            panic!("测试向量列数错误 {line}");
        };
        let device_id = u8::try_from(number(device_id)).unwrap();
        let target_id = u32::try_from(number(target_id)).unwrap();
        let address = match *target_format {
            "std" => {
                assert_eq!(target_id, u32::from(device_id), "{case}");
                Address::standard(device_id)
            }
            "ext" => Address::extended(target_id, device_id).unwrap(),
            _ => panic!("地址格式错误 {case}"),
        };
        let function = || u8::try_from(number(function)).unwrap();
        let request = || fixture_request(function(), number(value));
        let response = || fixture_response(function(), number(value));

        if *operation == "req_encode" {
            let error = encode_request(address, request()).unwrap_err();
            let actual = match error {
                EncodeError::InvalidExtendedCanId(_) => "invalid_extended_can_id",
                EncodeError::InvalidStandardAddress(_) => "invalid_standard_address",
                EncodeError::InvalidReportingPeriod(_) => "invalid_reporting_period",
            };
            assert_eq!(actual, *result, "{case}");
            continue;
        }

        let id = match *frame_format {
            "std" => FrameId::Standard(u16::try_from(number(frame_id)).unwrap()),
            "ext" => FrameId::Extended(u32::try_from(number(frame_id)).unwrap()),
            _ => panic!("帧 ID 格式错误 {case}"),
        };
        let mut bytes = Vec::new();
        let payload = match *frame_kind {
            "rtr" => FramePayload::Remote {
                dlc: u8::try_from(number(payload)).unwrap(),
            },
            "data" | "fd" => {
                if *payload != "-" {
                    assert_eq!(payload.len() % 2, 0, "{case}");
                    for pair in payload.as_bytes().chunks_exact(2) {
                        bytes.push(
                            u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap(),
                        );
                    }
                }
                if *frame_kind == "data" {
                    FramePayload::Data(&bytes)
                } else {
                    FramePayload::Fd(&bytes)
                }
            }
            _ => panic!("帧类型错误 {case}"),
        };
        let frame = FrameRef { id, payload };
        let encoded = match *operation {
            "req_both" => Some(encode_request(address, request()).unwrap()),
            "rsp_both" => Some(encode_response(address, response()).unwrap()),
            "req_decode" | "rsp_decode" => None,
            _ => panic!("操作类型错误 {case}"),
        };
        if let Some(encoded) = encoded {
            assert_eq!(encoded.as_ref(), frame, "{case}");
            assert_eq!(usize::from(encoded.dlc()), bytes.len(), "{case}");
        }

        // 从给定帧解码，验证结果独立于上面的编码结果。
        match *operation {
            "req_both" | "req_decode" => assert_eq!(
                decode_request(address, frame).map_err(decode_error_code),
                expected_decode(result, request),
                "{case}"
            ),
            "rsp_both" | "rsp_decode" => assert_eq!(
                decode_response(address, frame).map_err(decode_error_code),
                expected_decode(result, response),
                "{case}"
            ),
            _ => unreachable!(),
        }
    }
}

fn expected_decode<T>(result: &str, message: impl FnOnce() -> T) -> Result<Option<T>, &str> {
    match result {
        "ok" => Ok(Some(message())),
        "ignored" => Ok(None),
        error => Err(error),
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
fn extended_address_accessors_keep_can_id_and_device_id_independent() {
    let extended_address = Address::extended(0x18ff_f225, 0xa6).unwrap();
    assert_eq!(extended_address.id(), FrameId::Extended(0x18ff_f225));
    assert_eq!(extended_address.device_id(), 0xa6);
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
