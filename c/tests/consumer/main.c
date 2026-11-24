// Copyright The brt-can-protocol Contributors
// 用公共接口演示主机查询、设备应答和主动回传，并验证应用只采信位置样本。

#include <stdbool.h>

#include <brt_can_protocol/protocol.h>

static brt_frame_ref_t borrow_classic(const brt_encoded_frame_t *frame)
{
    return (brt_frame_ref_t){
        .id = frame->id,
        .kind = BRT_FRAME_CLASSIC_DATA,
        .data = frame->data,
        .len = frame->dlc,
    };
}

static bool accept_position(brt_address_t address, brt_frame_ref_t frame, uint32_t *count)
{
    brt_response_t response;
    if (brt_decode_response(address, frame, &response) != BRT_OK ||
        response.kind != BRT_RESPONSE_POSITION) {
        return false;
    }
    *count = response.value.position;
    return true;
}

static bool query_position(brt_address_t address)
{
    const brt_request_t query = {.kind = BRT_REQUEST_READ_POSITION};
    brt_encoded_frame_t query_frame;
    if (brt_encode_request(address, &query, &query_frame) != BRT_OK) {
        return false;
    }

    brt_request_t request;
    if (brt_decode_request(address, borrow_classic(&query_frame), &request) != BRT_OK ||
        request.kind != BRT_REQUEST_READ_POSITION) {
        return false;
    }

    const brt_response_t response = {.kind = BRT_RESPONSE_POSITION, .value.position = 74565};
    brt_encoded_frame_t reply_frame;
    if (brt_encode_response(address, &response, &reply_frame) != BRT_OK) {
        return false;
    }

    uint32_t count;
    return accept_position(address, borrow_classic(&reply_frame), &count) && count == 74565;
}

static bool receive_reports(brt_address_t address)
{
    const brt_response_t sample = {.kind = BRT_RESPONSE_POSITION, .value.position = UINT32_MAX};
    brt_encoded_frame_t report;
    uint32_t count = 0;
    if (brt_encode_response(address, &sample, &report) != BRT_OK ||
        !accept_position(address, borrow_classic(&report), &count) ||
        count != UINT32_MAX) {
        return false;
    }

    const uint8_t padded[] = {7, 1, 0x01, 0x45, 0x23, 0x01, 0, 0};
    const uint8_t wrong_len[] = {6, 1, 0x01, 0x45, 0x23, 0x01, 0};
    const uint8_t wrong_device[] = {7, 2, 0x01, 0x45, 0x23, 0x01, 0};
    const uint8_t position[] = {7, 1, 0x01, 0x45, 0x23, 0x01, 0};
    const uint8_t speed[] = {7, 1, 0x0a, 0, 0, 0, 0};
    const uint8_t ack[] = {4, 1, 0x05, 0};
    const brt_frame_ref_t rejected[] = {
        {address.id, BRT_FRAME_CLASSIC_DATA, padded, sizeof padded},
        {address.id, BRT_FRAME_CLASSIC_DATA, wrong_len, sizeof wrong_len},
        {address.id, BRT_FRAME_CLASSIC_DATA, wrong_device, sizeof wrong_device},
        {{BRT_ID_STANDARD, 2}, BRT_FRAME_CLASSIC_DATA, position, sizeof position},
        {address.id, BRT_FRAME_CLASSIC_DATA, speed, sizeof speed},
        {address.id, BRT_FRAME_CLASSIC_DATA, ack, sizeof ack},
    };
    for (size_t i = 0; i < sizeof rejected / sizeof rejected[0]; ++i) {
        if (accept_position(address, rejected[i], &count) || count != UINT32_MAX) {
            return false;
        }
    }
    return true;
}

int main(void)
{
    const brt_address_t address = {.id = {.format = BRT_ID_STANDARD, .value = 1}, .device_id = 1};
    if (!receive_reports(address)) {
        return 1;
    }
    if (!query_position(address)) {
        return 2;
    }
    return 0;
}
