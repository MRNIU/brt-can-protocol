/* Copyright The brt-can-protocol Contributors */
/* 实现 BRT CAN V2.7 帧校验及请求、应答的字节编解码。 */

#include <brt_can_protocol/protocol.h>

#include <stdbool.h>

#define MAX_STANDARD_CAN_ID UINT32_C(0x07ff)
#define MAX_EXTENDED_CAN_ID UINT32_C(0x1fffffff)
#define MIN_REPORTING_PERIOD_US 50u

static brt_result_t validate_id(brt_can_id_t id)
{
    switch (id.format) {
    case BRT_ID_STANDARD:
        return id.value <= MAX_STANDARD_CAN_ID ? BRT_OK : BRT_ERROR_INVALID_STANDARD_CAN_ID;
    case BRT_ID_EXTENDED:
        return id.value <= MAX_EXTENDED_CAN_ID ? BRT_OK : BRT_ERROR_INVALID_EXTENDED_CAN_ID;
    default:
        return BRT_ERROR_INVALID_ID_FORMAT;
    }
}

static brt_result_t validate_address(brt_address_t address)
{
    const brt_result_t result = validate_id(address.id);
    if (result != BRT_OK) {
        return result;
    }
    if (address.id.format == BRT_ID_STANDARD && address.id.value != address.device_id) {
        return BRT_ERROR_INVALID_ADDRESS;
    }
    return BRT_OK;
}

static brt_result_t validate_frame(brt_address_t address, brt_frame_ref_t frame, bool is_request)
{
    brt_result_t result = validate_address(address);
    if (result != BRT_OK) {
        return result;
    }
    result = validate_id(frame.id);
    if (result != BRT_OK) {
        return result;
    }
    switch (frame.kind) {
    case BRT_FRAME_CLASSIC_DATA:
        break;
    case BRT_FRAME_REMOTE:
        return BRT_ERROR_UNSUPPORTED_REMOTE_FRAME;
    case BRT_FRAME_FD:
        return BRT_ERROR_UNSUPPORTED_CAN_FD;
    default:
        return BRT_ERROR_INVALID_FRAME_KIND;
    }
    if (frame.id.format != address.id.format || frame.id.value != address.id.value) {
        return BRT_IGNORED;
    }
    if (frame.len > 8) {
        return BRT_ERROR_DATA_LENGTH_EXCEEDS_CLASSIC_CAN;
    }
    if (frame.len < 3) {
        return BRT_ERROR_INCOMPLETE_HEADER;
    }
    if (frame.data == NULL) {
        return BRT_ERROR_INVALID_ARGUMENT;
    }
    const uint8_t declared = frame.data[0];
    if (declared > 8) {
        return BRT_ERROR_DECLARED_LENGTH_EXCEEDS_CLASSIC_CAN;
    }
    /* 0x22 旧式请求的 LEN 为 4，但实际数据仍为 7 字节。 */
    const bool legacy_length = is_request && frame.data[2] == BRT_REQUEST_SET_EXTENDED_ADDRESS
                               && declared == 4 && frame.len == 7;
    if (declared != frame.len && !legacy_length) {
        return BRT_ERROR_LENGTH_MISMATCH;
    }
    if (frame.data[1] != address.device_id) {
        return BRT_ERROR_DEVICE_ID_MISMATCH;
    }
    return BRT_OK;
}

static uint8_t request_length(unsigned int function)
{
    switch (function) {
    case BRT_REQUEST_READ_POSITION:
    case BRT_REQUEST_SET_STANDARD_ADDRESS:
    case BRT_REQUEST_SET_BAUD_RATE:
    case BRT_REQUEST_SET_REPORTING_MODE:
    case BRT_REQUEST_SET_ZERO:
    case BRT_REQUEST_SET_DIRECTION:
    case BRT_REQUEST_READ_SPEED:
    case BRT_REQUEST_SET_MIDPOINT:
    case BRT_REQUEST_SET_FIVE_TURNS:
        return 4;
    case BRT_REQUEST_SET_REPORTING_PERIOD:
    case BRT_REQUEST_SET_SPEED_SAMPLE_TIME:
        return 5;
    case BRT_REQUEST_SET_EXTENDED_ADDRESS:
    case BRT_REQUEST_SET_POSITION:
        return 7;
    default:
        return 0;
    }
}

static bool is_ack_command(unsigned int function)
{
    switch (function) {
    case BRT_REQUEST_SET_STANDARD_ADDRESS:
    case BRT_REQUEST_SET_BAUD_RATE:
    case BRT_REQUEST_SET_REPORTING_MODE:
    case BRT_REQUEST_SET_REPORTING_PERIOD:
    case BRT_REQUEST_SET_ZERO:
    case BRT_REQUEST_SET_DIRECTION:
    case BRT_REQUEST_SET_SPEED_SAMPLE_TIME:
    case BRT_REQUEST_SET_MIDPOINT:
    case BRT_REQUEST_SET_POSITION:
    case BRT_REQUEST_SET_FIVE_TURNS:
        return true;
    default:
        return false;
    }
}

static bool valid_reporting_mode(unsigned int mode)
{
    switch (mode) {
    case BRT_REPORTING_QUERY:
    case BRT_REPORTING_STANDARD_POSITION:
    case BRT_REPORTING_STANDARD_SIGNED_SPEED:
    case BRT_REPORTING_STANDARD_UNSIGNED_SPEED:
    case BRT_REPORTING_EXTENDED_POSITION:
    case BRT_REPORTING_EXTENDED_SIGNED_SPEED:
    case BRT_REPORTING_EXTENDED_UNSIGNED_SPEED:
        return true;
    default:
        return false;
    }
}

static void put_u16_le(uint8_t *data, uint16_t value)
{
    data[0] = (uint8_t)value;
    data[1] = (uint8_t)(value >> 8);
}

static void put_u16_be(uint8_t *data, uint16_t value)
{
    data[0] = (uint8_t)(value >> 8);
    data[1] = (uint8_t)value;
}

static void put_u32_le(uint8_t *data, uint32_t value)
{
    data[0] = (uint8_t)value;
    data[1] = (uint8_t)(value >> 8);
    data[2] = (uint8_t)(value >> 16);
    data[3] = (uint8_t)(value >> 24);
}

static void put_u32_be(uint8_t *data, uint32_t value)
{
    data[0] = (uint8_t)(value >> 24);
    data[1] = (uint8_t)(value >> 16);
    data[2] = (uint8_t)(value >> 8);
    data[3] = (uint8_t)value;
}

static uint16_t read_u16_le(const uint8_t *data)
{
    return (uint16_t)((uint16_t)data[0] | ((uint16_t)data[1] << 8));
}

static uint16_t read_u16_be(const uint8_t *data)
{
    return (uint16_t)(((uint16_t)data[0] << 8) | (uint16_t)data[1]);
}

static uint32_t read_u32_le(const uint8_t *data)
{
    return (uint32_t)data[0] | ((uint32_t)data[1] << 8) | ((uint32_t)data[2] << 16)
           | ((uint32_t)data[3] << 24);
}

static uint32_t read_u32_be(const uint8_t *data)
{
    return ((uint32_t)data[0] << 24) | ((uint32_t)data[1] << 16) | ((uint32_t)data[2] << 8)
           | (uint32_t)data[3];
}

static int32_t read_i32_le(const uint8_t *data)
{
    const uint32_t bits = read_u32_le(data);
    const uint32_t positive_mask = (uint32_t)INT32_MAX;
    /* 为符号位赋予 INT32_MIN 的权值，所有到 int32_t 的转换均在范围内。 */
    return bits <= positive_mask ? (int32_t)bits : INT32_MIN + (int32_t)(bits & positive_mask);
}

brt_result_t brt_encode_request(brt_address_t address, const brt_request_t *request,
                                brt_encoded_frame_t *out)
{
    if (request == NULL || out == NULL) {
        return BRT_ERROR_INVALID_ARGUMENT;
    }
    const brt_result_t result = validate_address(address);
    if (result != BRT_OK) {
        return result;
    }
    const uint8_t length = request_length(request->kind);
    brt_encoded_frame_t encoded = {
        .id = address.id,
        .data = {length, address.device_id, (uint8_t)request->kind},
        .dlc = length,
    };
    uint8_t *data = &encoded.data[3];
    switch (request->kind) {
    case BRT_REQUEST_READ_POSITION:
    case BRT_REQUEST_SET_ZERO:
    case BRT_REQUEST_READ_SPEED:
        data[0] = 0;
        break;
    case BRT_REQUEST_SET_STANDARD_ADDRESS:
        if (request->value.standard_address == 0) {
            return BRT_ERROR_INVALID_STANDARD_ADDRESS;
        }
        data[0] = request->value.standard_address;
        break;
    case BRT_REQUEST_SET_EXTENDED_ADDRESS:
        if (request->value.extended_address > MAX_EXTENDED_CAN_ID) {
            return BRT_ERROR_INVALID_EXTENDED_CAN_ID;
        }
        put_u32_be(data, request->value.extended_address);
        break;
    case BRT_REQUEST_SET_BAUD_RATE:
        if ((unsigned int)request->value.baud_rate > BRT_BAUD_KBPS_100) {
            return BRT_ERROR_INVALID_BAUD_RATE;
        }
        data[0] = (uint8_t)request->value.baud_rate;
        break;
    case BRT_REQUEST_SET_REPORTING_MODE:
        if (!valid_reporting_mode(request->value.reporting_mode)) {
            return BRT_ERROR_INVALID_REPORTING_MODE;
        }
        data[0] = (uint8_t)request->value.reporting_mode;
        break;
    case BRT_REQUEST_SET_REPORTING_PERIOD:
        if (request->value.reporting_period_us < MIN_REPORTING_PERIOD_US) {
            return BRT_ERROR_INVALID_REPORTING_PERIOD;
        }
        put_u16_le(data, request->value.reporting_period_us);
        break;
    case BRT_REQUEST_SET_DIRECTION:
        if ((unsigned int)request->value.direction > BRT_DIRECTION_COUNTERCLOCKWISE) {
            return BRT_ERROR_INVALID_DIRECTION;
        }
        data[0] = (uint8_t)request->value.direction;
        break;
    case BRT_REQUEST_SET_SPEED_SAMPLE_TIME:
        put_u16_be(data, request->value.speed_sample_time_ms);
        break;
    case BRT_REQUEST_SET_MIDPOINT:
    case BRT_REQUEST_SET_FIVE_TURNS:
        data[0] = 1;
        break;
    case BRT_REQUEST_SET_POSITION:
        put_u32_be(data, request->value.position);
        break;
    default:
        return BRT_ERROR_INVALID_REQUEST_KIND;
    }
    *out = encoded;
    return BRT_OK;
}

brt_result_t brt_decode_request(brt_address_t address, brt_frame_ref_t frame, brt_request_t *out)
{
    if (out == NULL) {
        return BRT_ERROR_INVALID_ARGUMENT;
    }
    const brt_result_t result = validate_frame(address, frame, true);
    if (result != BRT_OK) {
        return result;
    }
    const uint8_t function = frame.data[2];
    const uint8_t length = request_length(function);
    if (length == 0) {
        return BRT_IGNORED;
    }
    if (frame.len != length) {
        return BRT_ERROR_INVALID_LENGTH;
    }
    brt_request_t decoded = {.kind = (brt_request_kind_t)function};
    const uint8_t *data = &frame.data[3];
    switch (decoded.kind) {
    case BRT_REQUEST_READ_POSITION:
    case BRT_REQUEST_SET_ZERO:
    case BRT_REQUEST_READ_SPEED:
        if (data[0] != 0) {
            return BRT_ERROR_INVALID_FIXED_PARAMETER;
        }
        break;
    case BRT_REQUEST_SET_STANDARD_ADDRESS:
        if (data[0] == 0) {
            return BRT_ERROR_INVALID_STANDARD_ADDRESS;
        }
        decoded.value.standard_address = data[0];
        break;
    case BRT_REQUEST_SET_EXTENDED_ADDRESS:
        decoded.value.extended_address = read_u32_be(data);
        if (decoded.value.extended_address > MAX_EXTENDED_CAN_ID) {
            return BRT_ERROR_INVALID_EXTENDED_CAN_ID;
        }
        break;
    case BRT_REQUEST_SET_BAUD_RATE:
        if (data[0] > BRT_BAUD_KBPS_100) {
            return BRT_ERROR_INVALID_BAUD_RATE;
        }
        decoded.value.baud_rate = (brt_baud_rate_t)data[0];
        break;
    case BRT_REQUEST_SET_REPORTING_MODE:
        if (!valid_reporting_mode(data[0])) {
            return BRT_ERROR_INVALID_REPORTING_MODE;
        }
        decoded.value.reporting_mode = (brt_reporting_mode_t)data[0];
        break;
    case BRT_REQUEST_SET_REPORTING_PERIOD:
        decoded.value.reporting_period_us = read_u16_le(data);
        if (decoded.value.reporting_period_us < MIN_REPORTING_PERIOD_US) {
            return BRT_ERROR_INVALID_REPORTING_PERIOD;
        }
        break;
    case BRT_REQUEST_SET_DIRECTION:
        if (data[0] > BRT_DIRECTION_COUNTERCLOCKWISE) {
            return BRT_ERROR_INVALID_DIRECTION;
        }
        decoded.value.direction = (brt_direction_t)data[0];
        break;
    case BRT_REQUEST_SET_SPEED_SAMPLE_TIME:
        decoded.value.speed_sample_time_ms = read_u16_be(data);
        break;
    case BRT_REQUEST_SET_MIDPOINT:
    case BRT_REQUEST_SET_FIVE_TURNS:
        if (data[0] != 1) {
            return BRT_ERROR_INVALID_FIXED_PARAMETER;
        }
        break;
    case BRT_REQUEST_SET_POSITION:
        decoded.value.position = read_u32_be(data);
        break;
    }
    *out = decoded;
    return BRT_OK;
}

brt_result_t brt_encode_response(brt_address_t address, const brt_response_t *response,
                                 brt_encoded_frame_t *out)
{
    if (response == NULL || out == NULL) {
        return BRT_ERROR_INVALID_ARGUMENT;
    }
    const brt_result_t result = validate_address(address);
    if (result != BRT_OK) {
        return result;
    }
    brt_encoded_frame_t encoded = {.id = address.id, .dlc = 7};
    uint8_t *data = encoded.data;
    switch (response->kind) {
    case BRT_RESPONSE_POSITION:
        data[2] = BRT_REQUEST_READ_POSITION;
        put_u32_le(&data[3], response->value.position);
        break;
    case BRT_RESPONSE_SPEED:
        data[2] = BRT_REQUEST_READ_SPEED;
        put_u32_le(&data[3], (uint32_t)response->value.speed);
        break;
    case BRT_RESPONSE_ACK:
        if (!is_ack_command(response->value.ack.command)) {
            return BRT_ERROR_INVALID_ACK_COMMAND;
        }
        encoded.dlc = 4;
        data[2] = (uint8_t)response->value.ack.command;
        data[3] = response->value.ack.status;
        break;
    case BRT_RESPONSE_SET_EXTENDED_ADDRESS:
        data[2] = BRT_REQUEST_SET_EXTENDED_ADDRESS;
        put_u32_be(&data[3], response->value.extended_address_status);
        break;
    default:
        return BRT_ERROR_INVALID_RESPONSE_KIND;
    }
    data[0] = encoded.dlc;
    data[1] = address.device_id;
    *out = encoded;
    return BRT_OK;
}

brt_result_t brt_decode_response(brt_address_t address, brt_frame_ref_t frame, brt_response_t *out)
{
    if (out == NULL) {
        return BRT_ERROR_INVALID_ARGUMENT;
    }
    const brt_result_t result = validate_frame(address, frame, false);
    if (result != BRT_OK) {
        return result;
    }
    const uint8_t function = frame.data[2];
    const bool is_ack = is_ack_command(function);
    if (!is_ack && function != BRT_REQUEST_READ_POSITION && function != BRT_REQUEST_READ_SPEED
        && function != BRT_REQUEST_SET_EXTENDED_ADDRESS) {
        return BRT_IGNORED;
    }
    const uint8_t length = is_ack ? 4 : 7;
    if (frame.len != length) {
        return BRT_ERROR_INVALID_LENGTH;
    }
    const uint8_t *data = &frame.data[3];
    brt_response_t decoded;
    switch (function) {
    case BRT_REQUEST_READ_POSITION:
        decoded = (brt_response_t){.kind = BRT_RESPONSE_POSITION,
                                   .value.position = read_u32_le(data)};
        break;
    case BRT_REQUEST_READ_SPEED:
        decoded = (brt_response_t){.kind = BRT_RESPONSE_SPEED,
                                   .value.speed = read_i32_le(data)};
        break;
    case BRT_REQUEST_SET_EXTENDED_ADDRESS:
        decoded = (brt_response_t){.kind = BRT_RESPONSE_SET_EXTENDED_ADDRESS,
                                   .value.extended_address_status = read_u32_be(data)};
        break;
    default:
        decoded = (brt_response_t){
            .kind = BRT_RESPONSE_ACK,
            .value.ack = {.command = (brt_request_kind_t)function, .status = data[0]},
        };
        break;
    }
    *out = decoded;
    return BRT_OK;
}
