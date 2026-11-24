/* Copyright The brt-can-protocol Contributors */
/* 用独立协议向量验证 C 编解码和 C 接口参数边界。 */

#include "brt_can_protocol/protocol.h"

#include <errno.h>
#include <inttypes.h>
#include <stdbool.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/* 共享向量按数值验证报文。用独立断言固定 C 调用方使用的枚举名称与线上值。 */
#define CHECK_WIRE_VALUE(name, value) _Static_assert((name) == (value), #name " 的线上值应为 " #value)

CHECK_WIRE_VALUE(BRT_REQUEST_READ_POSITION, 0x01);
CHECK_WIRE_VALUE(BRT_REQUEST_SET_STANDARD_ADDRESS, 0x02);
CHECK_WIRE_VALUE(BRT_REQUEST_SET_EXTENDED_ADDRESS, 0x22);
CHECK_WIRE_VALUE(BRT_REQUEST_SET_BAUD_RATE, 0x03);
CHECK_WIRE_VALUE(BRT_REQUEST_SET_REPORTING_MODE, 0x04);
CHECK_WIRE_VALUE(BRT_REQUEST_SET_REPORTING_PERIOD, 0x05);
CHECK_WIRE_VALUE(BRT_REQUEST_SET_ZERO, 0x06);
CHECK_WIRE_VALUE(BRT_REQUEST_SET_DIRECTION, 0x07);
CHECK_WIRE_VALUE(BRT_REQUEST_READ_SPEED, 0x0a);
CHECK_WIRE_VALUE(BRT_REQUEST_SET_SPEED_SAMPLE_TIME, 0x0b);
CHECK_WIRE_VALUE(BRT_REQUEST_SET_MIDPOINT, 0x0c);
CHECK_WIRE_VALUE(BRT_REQUEST_SET_POSITION, 0x0d);
CHECK_WIRE_VALUE(BRT_REQUEST_SET_FIVE_TURNS, 0x0f);

CHECK_WIRE_VALUE(BRT_BAUD_KBPS_500, 0x00);
CHECK_WIRE_VALUE(BRT_BAUD_MBPS_1, 0x01);
CHECK_WIRE_VALUE(BRT_BAUD_KBPS_250, 0x02);
CHECK_WIRE_VALUE(BRT_BAUD_KBPS_125, 0x03);
CHECK_WIRE_VALUE(BRT_BAUD_KBPS_100, 0x04);

CHECK_WIRE_VALUE(BRT_REPORTING_QUERY, 0x00);
CHECK_WIRE_VALUE(BRT_REPORTING_STANDARD_POSITION, 0xaa);
CHECK_WIRE_VALUE(BRT_REPORTING_STANDARD_SIGNED_SPEED, 0x02);
CHECK_WIRE_VALUE(BRT_REPORTING_STANDARD_UNSIGNED_SPEED, 0x07);
CHECK_WIRE_VALUE(BRT_REPORTING_EXTENDED_POSITION, 0x18);
CHECK_WIRE_VALUE(BRT_REPORTING_EXTENDED_SIGNED_SPEED, 0x12);
CHECK_WIRE_VALUE(BRT_REPORTING_EXTENDED_UNSIGNED_SPEED, 0x17);

CHECK_WIRE_VALUE(BRT_DIRECTION_CLOCKWISE, 0x00);
CHECK_WIRE_VALUE(BRT_DIRECTION_COUNTERCLOCKWISE, 0x01);

#undef CHECK_WIRE_VALUE

typedef enum {
    CASE_REQUEST_BOTH,
    CASE_RESPONSE_BOTH,
    CASE_REQUEST_DECODE,
    CASE_RESPONSE_DECODE,
    CASE_REQUEST_ENCODE
} operation_t;

typedef struct {
    const char *name;
    size_t line;
    operation_t operation;
    brt_address_t address;
    brt_frame_ref_t frame;
    uint8_t data[64];
    uint8_t function;
    int64_t value;
    brt_result_t result;
} test_case_t;

/* Release 构建也需要执行全部断言。 */
#define CHECK(test, condition)                                                   \
    do {                                                                         \
        if (!(condition)) {                                                      \
            fprintf(stderr, "%s（向量行 %zu，测试代码行 %d）：%s\n",             \
                    (test)->name, (test)->line, __LINE__, #condition);             \
            exit(EXIT_FAILURE);                                                  \
        }                                                                        \
    } while (false)

static int64_t number(const test_case_t *test, const char *text, int base,
                      int64_t min, int64_t max)
{
    char *end;
    errno = 0;
    intmax_t value = strtoimax(text, &end, base);
    CHECK(test, errno == 0 && end != text && *end == '\0' && value >= min && value <= max);
    return (int64_t)value;
}

static brt_can_id_t read_can_id(const test_case_t *test, const char *format, const char *id)
{
    CHECK(test, strcmp(format, "std") == 0 || strcmp(format, "ext") == 0);
    return (brt_can_id_t){
        .format = strcmp(format, "std") == 0 ? BRT_ID_STANDARD : BRT_ID_EXTENDED,
        .value = (uint32_t)number(test, id, 0, 0, UINT32_MAX)
    };
}

static operation_t read_operation(const test_case_t *test, const char *name)
{
    static const struct {
        const char *name;
        operation_t operation;
    } operations[] = {
        {"req_both", CASE_REQUEST_BOTH},
        {"rsp_both", CASE_RESPONSE_BOTH},
        {"req_decode", CASE_REQUEST_DECODE},
        {"rsp_decode", CASE_RESPONSE_DECODE},
        {"req_encode", CASE_REQUEST_ENCODE}
    };
    for (size_t i = 0; i < sizeof(operations) / sizeof(operations[0]); ++i) {
        if (strcmp(name, operations[i].name) == 0) {
            return operations[i].operation;
        }
    }
    CHECK(test, false);
    return CASE_REQUEST_BOTH;
}

static brt_result_t read_result(const test_case_t *test, const char *name)
{
    static const struct {
        const char *name;
        brt_result_t result;
    } results[] = {
        {"ok", BRT_OK},
        {"ignored", BRT_IGNORED},
        {"invalid_standard_can_id", BRT_ERROR_INVALID_STANDARD_CAN_ID},
        {"invalid_extended_can_id", BRT_ERROR_INVALID_EXTENDED_CAN_ID},
        {"unsupported_remote_frame", BRT_ERROR_UNSUPPORTED_REMOTE_FRAME},
        {"unsupported_can_fd", BRT_ERROR_UNSUPPORTED_CAN_FD},
        {"data_length_exceeds_classic_can", BRT_ERROR_DATA_LENGTH_EXCEEDS_CLASSIC_CAN},
        {"incomplete_header", BRT_ERROR_INCOMPLETE_HEADER},
        {"length_mismatch", BRT_ERROR_LENGTH_MISMATCH},
        {"declared_length_exceeds_classic_can", BRT_ERROR_DECLARED_LENGTH_EXCEEDS_CLASSIC_CAN},
        {"device_id_mismatch", BRT_ERROR_DEVICE_ID_MISMATCH},
        {"invalid_length", BRT_ERROR_INVALID_LENGTH},
        {"invalid_fixed_parameter", BRT_ERROR_INVALID_FIXED_PARAMETER},
        {"invalid_standard_address", BRT_ERROR_INVALID_STANDARD_ADDRESS},
        {"invalid_baud_rate", BRT_ERROR_INVALID_BAUD_RATE},
        {"invalid_reporting_mode", BRT_ERROR_INVALID_REPORTING_MODE},
        {"invalid_direction", BRT_ERROR_INVALID_DIRECTION},
        {"invalid_reporting_period", BRT_ERROR_INVALID_REPORTING_PERIOD}
    };
    for (size_t i = 0; i < sizeof(results) / sizeof(results[0]); ++i) {
        if (strcmp(name, results[i].name) == 0) {
            return results[i].result;
        }
    }
    CHECK(test, false);
    return BRT_OK;
}

static void read_frame(test_case_t *test, const char *format, const char *id,
                       const char *kind, const char *payload)
{
    test->frame.id = read_can_id(test, format, id);
    if (strcmp(kind, "rtr") == 0) {
        test->frame.kind = BRT_FRAME_REMOTE;
        test->frame.len = (size_t)number(test, payload, 10, 0, UINT8_MAX);
        return;
    }

    CHECK(test, strcmp(kind, "data") == 0 || strcmp(kind, "fd") == 0);
    test->frame.kind = strcmp(kind, "data") == 0 ? BRT_FRAME_CLASSIC_DATA : BRT_FRAME_FD;
    if (strcmp(payload, "-") == 0) {
        return;
    }
    size_t digits = strlen(payload);
    CHECK(test, digits % 2 == 0 && digits / 2 <= sizeof(test->data));
    CHECK(test, strspn(payload, "0123456789abcdefABCDEF") == digits);
    for (size_t i = 0; i < digits / 2; ++i) {
        const char byte[] = {payload[2 * i], payload[2 * i + 1], '\0'};
        test->data[i] = (uint8_t)number(test, byte, 16, 0, UINT8_MAX);
    }
    test->frame.data = test->data;
    test->frame.len = digits / 2;
}

static void read_case(test_case_t *test, char *line)
{
    enum {
        NAME, OPERATION, TARGET_FORMAT, TARGET_ID, DEVICE_ID,
        FRAME_FORMAT, FRAME_ID, FRAME_KIND, PAYLOAD, FUNCTION, VALUE, RESULT, COLUMN_COUNT
    };
    const char *columns[COLUMN_COUNT];
    test->name = line;
    for (size_t i = 0; i < COLUMN_COUNT; ++i) {
        columns[i] = line;
        char *tab = strchr(line, '\t');
        CHECK(test, (tab != NULL) == (i + 1 < COLUMN_COUNT));
        if (tab != NULL) {
            *tab = '\0';
            line = tab + 1;
        }
        CHECK(test, columns[i][0] != '\0');
    }

    test->name = columns[NAME];
    test->operation = read_operation(test, columns[OPERATION]);
    test->address = (brt_address_t){
        .id = read_can_id(test, columns[TARGET_FORMAT], columns[TARGET_ID]),
        .device_id = (uint8_t)number(test, columns[DEVICE_ID], 0, 0, UINT8_MAX)
    };
    test->result = read_result(test, columns[RESULT]);
    if (test->operation == CASE_REQUEST_ENCODE) {
        for (size_t i = FRAME_FORMAT; i <= PAYLOAD; ++i) {
            CHECK(test, strcmp(columns[i], "-") == 0);
        }
        CHECK(test, test->result != BRT_OK && test->result != BRT_IGNORED);
    } else {
        read_frame(test, columns[FRAME_FORMAT], columns[FRAME_ID], columns[FRAME_KIND],
                   columns[PAYLOAD]);
    }
    if (test->result == BRT_OK || test->operation == CASE_REQUEST_ENCODE) {
        test->function = (uint8_t)number(test, columns[FUNCTION], 0, 0, UINT8_MAX);
        test->value = number(test, columns[VALUE], 0, INT32_MIN, UINT32_MAX);
    } else {
        CHECK(test, strcmp(columns[FUNCTION], "-") == 0 && strcmp(columns[VALUE], "-") == 0);
    }
}

static uint32_t unsigned_value(const test_case_t *test, uint32_t max)
{
    CHECK(test, test->value >= 0 && test->value <= max);
    return (uint32_t)test->value;
}

static brt_request_t make_request(const test_case_t *test)
{
    brt_request_t request = {.kind = (brt_request_kind_t)test->function};
    switch (request.kind) {
    case BRT_REQUEST_READ_POSITION:
    case BRT_REQUEST_SET_ZERO:
    case BRT_REQUEST_READ_SPEED:
    case BRT_REQUEST_SET_MIDPOINT:
    case BRT_REQUEST_SET_FIVE_TURNS:
        CHECK(test, test->value == 0);
        break;
    case BRT_REQUEST_SET_STANDARD_ADDRESS:
        request.value.standard_address = (uint8_t)unsigned_value(test, UINT8_MAX);
        break;
    case BRT_REQUEST_SET_EXTENDED_ADDRESS:
        request.value.extended_address = unsigned_value(test, UINT32_MAX);
        break;
    case BRT_REQUEST_SET_BAUD_RATE:
        request.value.baud_rate = (brt_baud_rate_t)unsigned_value(test, UINT8_MAX);
        break;
    case BRT_REQUEST_SET_REPORTING_MODE:
        request.value.reporting_mode = (brt_reporting_mode_t)unsigned_value(test, UINT8_MAX);
        break;
    case BRT_REQUEST_SET_REPORTING_PERIOD:
        request.value.reporting_period_us = (uint16_t)unsigned_value(test, UINT16_MAX);
        break;
    case BRT_REQUEST_SET_DIRECTION:
        request.value.direction = (brt_direction_t)unsigned_value(test, UINT8_MAX);
        break;
    case BRT_REQUEST_SET_SPEED_SAMPLE_TIME:
        request.value.speed_sample_time_ms = (uint16_t)unsigned_value(test, UINT16_MAX);
        break;
    case BRT_REQUEST_SET_POSITION:
        request.value.position = unsigned_value(test, UINT32_MAX);
        break;
    default:
        CHECK(test, false);
    }
    return request;
}

static int64_t request_value(const test_case_t *test, const brt_request_t *request)
{
    switch (request->kind) {
    case BRT_REQUEST_READ_POSITION:
    case BRT_REQUEST_SET_ZERO:
    case BRT_REQUEST_READ_SPEED:
    case BRT_REQUEST_SET_MIDPOINT:
    case BRT_REQUEST_SET_FIVE_TURNS:
        return 0;
    case BRT_REQUEST_SET_STANDARD_ADDRESS:
        return request->value.standard_address;
    case BRT_REQUEST_SET_EXTENDED_ADDRESS:
        return request->value.extended_address;
    case BRT_REQUEST_SET_BAUD_RATE:
        return request->value.baud_rate;
    case BRT_REQUEST_SET_REPORTING_MODE:
        return request->value.reporting_mode;
    case BRT_REQUEST_SET_REPORTING_PERIOD:
        return request->value.reporting_period_us;
    case BRT_REQUEST_SET_DIRECTION:
        return request->value.direction;
    case BRT_REQUEST_SET_SPEED_SAMPLE_TIME:
        return request->value.speed_sample_time_ms;
    case BRT_REQUEST_SET_POSITION:
        return request->value.position;
    default:
        CHECK(test, false);
        return 0;
    }
}

static brt_response_t make_response(const test_case_t *test)
{
    switch (test->function) {
    case 0x01:
        return (brt_response_t){
            .kind = BRT_RESPONSE_POSITION,
            .value.position = unsigned_value(test, UINT32_MAX)
        };
    case 0x0a:
        CHECK(test, test->value <= INT32_MAX);
        return (brt_response_t){.kind = BRT_RESPONSE_SPEED, .value.speed = (int32_t)test->value};
    case 0x22:
        return (brt_response_t){
            .kind = BRT_RESPONSE_SET_EXTENDED_ADDRESS,
            .value.extended_address_status = unsigned_value(test, UINT32_MAX)
        };
    default:
        return (brt_response_t){
            .kind = BRT_RESPONSE_ACK,
            .value.ack = {
                .command = (brt_request_kind_t)test->function,
                .status = (uint8_t)unsigned_value(test, UINT8_MAX)
            }
        };
    }
}

static void check_response_value(const test_case_t *test, const brt_response_t *response)
{
    switch (response->kind) {
    case BRT_RESPONSE_POSITION:
        CHECK(test, test->function == 0x01 && response->value.position == test->value);
        break;
    case BRT_RESPONSE_SPEED:
        CHECK(test, test->function == 0x0a && response->value.speed == test->value);
        break;
    case BRT_RESPONSE_SET_EXTENDED_ADDRESS:
        CHECK(test, test->function == 0x22 && response->value.extended_address_status == test->value);
        break;
    case BRT_RESPONSE_ACK:
        CHECK(test, test->function != 0x01 && test->function != 0x0a && test->function != 0x22);
        CHECK(test, response->value.ack.command == (brt_request_kind_t)test->function);
        CHECK(test, response->value.ack.status == test->value);
        break;
    default:
        CHECK(test, false);
    }
}

static void check_encoded_frame(const test_case_t *test, const brt_encoded_frame_t *frame)
{
    CHECK(test, frame->id.format == test->frame.id.format && frame->id.value == test->frame.id.value);
    CHECK(test, test->frame.kind == BRT_FRAME_CLASSIC_DATA);
    CHECK(test, frame->dlc == test->frame.len && frame->dlc <= sizeof(frame->data));
    CHECK(test, memcmp(frame->data, test->data, frame->dlc) == 0);
    for (size_t i = frame->dlc; i < sizeof(frame->data); ++i) {
        CHECK(test, frame->data[i] == 0);
    }
}

enum { OUTPUT_SENTINEL = 0xa5 };

static void check_untouched(const test_case_t *test, const void *output, size_t size)
{
    const unsigned char *bytes = output;
    for (size_t i = 0; i < size; ++i) {
        CHECK(test, bytes[i] == OUTPUT_SENTINEL);
    }
}

static void check_request_vector(const test_case_t *test)
{
    if (test->operation != CASE_REQUEST_DECODE) {
        brt_request_t request = make_request(test);
        brt_encoded_frame_t frame;
        memset(&frame, OUTPUT_SENTINEL, sizeof(frame));
        CHECK(test, brt_encode_request(test->address, &request, &frame) == test->result);
        if (test->result == BRT_OK) {
            check_encoded_frame(test, &frame);
        } else {
            check_untouched(test, &frame, sizeof(frame));
        }
    }
    if (test->operation != CASE_REQUEST_ENCODE) {
        /* 双向向量的解码直接使用独立给定的线上字节。 */
        brt_request_t request;
        memset(&request, OUTPUT_SENTINEL, sizeof(request));
        CHECK(test, brt_decode_request(test->address, test->frame, &request) == test->result);
        if (test->result == BRT_OK) {
            CHECK(test, request.kind == (brt_request_kind_t)test->function);
            CHECK(test, request_value(test, &request) == test->value);
        } else {
            check_untouched(test, &request, sizeof(request));
        }
    }
}

static void check_response_vector(const test_case_t *test)
{
    if (test->operation == CASE_RESPONSE_BOTH) {
        const brt_response_t response = make_response(test);
        brt_encoded_frame_t frame;
        memset(&frame, OUTPUT_SENTINEL, sizeof(frame));
        CHECK(test, brt_encode_response(test->address, &response, &frame) == BRT_OK);
        check_encoded_frame(test, &frame);
    }

    brt_response_t response;
    memset(&response, OUTPUT_SENTINEL, sizeof(response));
    CHECK(test, brt_decode_response(test->address, test->frame, &response) == test->result);
    if (test->result == BRT_OK) {
        check_response_value(test, &response);
    } else {
        check_untouched(test, &response, sizeof(response));
    }
}

static void check_c_inputs(void)
{
    const test_case_t test = {.name = "C 接口参数"};
    const brt_address_t address = {.id = {BRT_ID_STANDARD, 1}, .device_id = 1};
    const brt_request_t query = {.kind = BRT_REQUEST_READ_POSITION};
    const brt_response_t position = {.kind = BRT_RESPONSE_POSITION, .value.position = 74565};
    brt_frame_ref_t frame = {.id = address.id, .kind = BRT_FRAME_CLASSIC_DATA, .len = 7};
    brt_encoded_frame_t encoded;
    brt_request_t request;
    brt_response_t response;
    memset(&encoded, OUTPUT_SENTINEL, sizeof(encoded));
    memset(&request, OUTPUT_SENTINEL, sizeof(request));
    memset(&response, OUTPUT_SENTINEL, sizeof(response));

    CHECK(&test, brt_encode_request(address, NULL, &encoded) == BRT_ERROR_INVALID_ARGUMENT);
    CHECK(&test, brt_encode_response(address, NULL, &encoded) == BRT_ERROR_INVALID_ARGUMENT);
    CHECK(&test, brt_encode_request(address, &query, NULL) == BRT_ERROR_INVALID_ARGUMENT);
    CHECK(&test, brt_encode_response(address, &position, NULL) == BRT_ERROR_INVALID_ARGUMENT);
    CHECK(&test, brt_decode_request(address, frame, NULL) == BRT_ERROR_INVALID_ARGUMENT);
    CHECK(&test, brt_decode_response(address, frame, NULL) == BRT_ERROR_INVALID_ARGUMENT);
    CHECK(&test, brt_decode_request(address, frame, &request) == BRT_ERROR_INVALID_ARGUMENT);
    CHECK(&test, brt_decode_response(address, frame, &response) == BRT_ERROR_INVALID_ARGUMENT);

    frame.len = 0;
    CHECK(&test, brt_decode_response(address, frame, &response) == BRT_ERROR_INCOMPLETE_HEADER);

    /* 使用恰好等于有效长度的缓冲区，让 ASan 能发现越界读取。 */
    const uint8_t short_header[] = {7, 1};
    frame.data = short_header;
    frame.len = sizeof(short_header);
    CHECK(&test, brt_decode_response(address, frame, &response) == BRT_ERROR_INCOMPLETE_HEADER);
    const uint8_t no_position[] = {3, 1, 1};
    frame.data = no_position;
    frame.len = sizeof(no_position);
    CHECK(&test, brt_decode_response(address, frame, &response) == BRT_ERROR_INVALID_LENGTH);

    frame.id.value = 2;
    frame.data = NULL;
    frame.len = 7;
    CHECK(&test, brt_decode_response(address, frame, &response) == BRT_IGNORED);

    frame.id.format = (brt_id_format_t)0xff;
    CHECK(&test, brt_decode_request(address, frame, &request) == BRT_ERROR_INVALID_ID_FORMAT);
    frame.id = address.id;
    frame.kind = (brt_frame_kind_t)0xff;
    CHECK(&test, brt_decode_response(address, frame, &response) == BRT_ERROR_INVALID_FRAME_KIND);

    brt_address_t invalid_address = address;
    invalid_address.id.value = 2;
    CHECK(&test, brt_encode_request(invalid_address, &query, &encoded) == BRT_ERROR_INVALID_ADDRESS);
    CHECK(&test, brt_encode_response(invalid_address, &position, &encoded) == BRT_ERROR_INVALID_ADDRESS);
    frame.kind = BRT_FRAME_CLASSIC_DATA;
    CHECK(&test, brt_decode_request(invalid_address, frame, &request) == BRT_ERROR_INVALID_ADDRESS);
    CHECK(&test, brt_decode_response(invalid_address, frame, &response) == BRT_ERROR_INVALID_ADDRESS);
    invalid_address.id.format = (brt_id_format_t)0xff;
    CHECK(&test, brt_encode_request(invalid_address, &query, &encoded) == BRT_ERROR_INVALID_ID_FORMAT);

    const brt_request_t invalid_request = {.kind = (brt_request_kind_t)0xff};
    CHECK(&test, brt_encode_request(address, &invalid_request, &encoded) == BRT_ERROR_INVALID_REQUEST_KIND);
    brt_response_t invalid_response = {.kind = (brt_response_kind_t)0xff};
    CHECK(&test, brt_encode_response(address, &invalid_response, &encoded) == BRT_ERROR_INVALID_RESPONSE_KIND);
    invalid_response.kind = BRT_RESPONSE_ACK;
    const brt_request_kind_t invalid_ack_commands[] = {
        BRT_REQUEST_READ_POSITION,
        BRT_REQUEST_READ_SPEED,
        BRT_REQUEST_SET_EXTENDED_ADDRESS,
        (brt_request_kind_t)0xff,
    };
    for (size_t i = 0; i < sizeof(invalid_ack_commands) / sizeof(invalid_ack_commands[0]); ++i) {
        invalid_response.value.ack = (brt_ack_t){.command = invalid_ack_commands[i]};
        CHECK(&test, brt_encode_response(address, &invalid_response, &encoded) == BRT_ERROR_INVALID_ACK_COMMAND);
    }

    const brt_request_t invalid_values[] = {
        {.kind = BRT_REQUEST_SET_BAUD_RATE, .value.baud_rate = (brt_baud_rate_t)0xff},
        {.kind = BRT_REQUEST_SET_REPORTING_MODE, .value.reporting_mode = (brt_reporting_mode_t)0xff},
        {.kind = BRT_REQUEST_SET_DIRECTION, .value.direction = (brt_direction_t)0xff},
    };
    const brt_result_t invalid_results[] = {
        BRT_ERROR_INVALID_BAUD_RATE,
        BRT_ERROR_INVALID_REPORTING_MODE,
        BRT_ERROR_INVALID_DIRECTION,
    };
    for (size_t i = 0; i < sizeof(invalid_values) / sizeof(invalid_values[0]); ++i) {
        CHECK(&test, brt_encode_request(address, &invalid_values[i], &encoded) == invalid_results[i]);
    }
    check_untouched(&test, &encoded, sizeof(encoded));
    check_untouched(&test, &request, sizeof(request));
    check_untouched(&test, &response, sizeof(response));
}

int main(int argc, char **argv)
{
    if (argc != 2) {
        fputs("需指定共享协议向量路径\n", stderr);
        return EXIT_FAILURE;
    }
    FILE *vectors = fopen(argv[1], "rb");
    if (vectors == NULL) {
        perror(argv[1]);
        return EXIT_FAILURE;
    }

    size_t line_number = 0;
    size_t count = 0;
    char line[512];
    while (fgets(line, sizeof(line), vectors) != NULL) {
        test_case_t test = {.name = "读取向量", .line = ++line_number};
        size_t end = strcspn(line, "\r\n");
        CHECK(&test, line[end] != '\0' || feof(vectors));
        line[end] = '\0';
        if (line[0] == '#' || line[0] == '\0') {
            continue;
        }

        read_case(&test, line);
        if (test.operation == CASE_RESPONSE_BOTH || test.operation == CASE_RESPONSE_DECODE) {
            check_response_vector(&test);
        } else {
            check_request_vector(&test);
        }
        ++count;
    }
    const bool read_failed = ferror(vectors) != 0;
    if (fclose(vectors) != 0 || read_failed || count == 0) {
        fprintf(stderr, "无法完整读取协议向量 %s\n", argv[1]);
        return EXIT_FAILURE;
    }

    check_c_inputs();
    printf("通过 %zu 条共享协议向量和 C 接口参数测试\n", count);
    return EXIT_SUCCESS;
}
