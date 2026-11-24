/* Copyright The brt-can-protocol Contributors */

/**
 * @file
 * @brief 布瑞特 BRT CAN V2.7 无状态编解码接口。
 *
 * 调用方负责设备访问、收发和地址变化后的匹配。协议依据见仓库 README。
 */

#ifndef BRT_CAN_PROTOCOL_PROTOCOL_H
#define BRT_CAN_PROTOCOL_PROTOCOL_H

#include <stddef.h>
#include <stdint.h>

/** CAN 标识符的格式。 */
typedef enum {
    BRT_ID_STANDARD, /**< 11 位标准标识符。 */
    BRT_ID_EXTENDED, /**< 29 位扩展标识符。 */
} brt_id_format_t;

/** CAN 标识符及其格式。 */
typedef struct {
    brt_id_format_t format; /**< 标识符格式。 */
    uint32_t value; /**< 保留原始数值，编解码器检查格式位宽。 */
} brt_can_id_t;

/**
 * 单次调用使用的协议地址。
 *
 * 标准地址的 CAN ID 与八位设备号相等，允许当前地址为 0。
 * 扩展地址的 29 位 CAN ID 与设备号均由调用方给定，库分别保留两者。
 */
typedef struct {
    brt_can_id_t id; /**< 预期的 CAN 标识符及格式。 */
    uint8_t device_id; /**< 数据域中的 DEVICE_ID。 */
} brt_address_t;

/** 接收帧的实际帧形态。 */
typedef enum {
    BRT_FRAME_CLASSIC_DATA, /**< Classic CAN 数据帧。 */
    BRT_FRAME_REMOTE, /**< Classic CAN RTR 帧。 */
    BRT_FRAME_FD, /**< CAN FD 数据帧，短帧也使用此类型。 */
} brt_frame_kind_t;

/**
 * 解码期间借用的接收帧。
 *
 * 对 Classic 数据帧，调用方先核对原生 DLC 和缓冲区容量，传实际有效字节数。
 * 解码器只访问地址匹配且长度在 3 到 8 字节的 Classic 数据缓冲区。此时空指针返回
 * BRT_ERROR_INVALID_ARGUMENT。其余帧可使用空指针。
 */
typedef struct {
    brt_can_id_t id; /**< 原始 CAN 标识符及格式。 */
    brt_frame_kind_t kind; /**< 驱动报告的实际帧形态。 */
    const uint8_t *data; /**< 解码时可访问的 Classic 数据缓冲区，须覆盖有效的 len 字节。 */
    size_t len; /**< 数据帧有效字节数或 RTR 报告的 DLC。 */
} brt_frame_ref_t;

/** 编码产生的 Classic CAN 数据帧。 */
typedef struct {
    brt_can_id_t id; /**< CAN 标识符及格式。 */
    uint8_t data[8]; /**< 固定容量数据缓冲区，未使用尾部为零。 */
    uint8_t dlc; /**< 有效字节数，发送仅使用此前缀。 */
} brt_encoded_frame_t;

/** 波特率配置，枚举值等于线上参数。 */
typedef enum {
    BRT_BAUD_KBPS_500 = 0x00, /**< 500 kbit/s。 */
    BRT_BAUD_MBPS_1 = 0x01, /**< 1 Mbit/s。 */
    BRT_BAUD_KBPS_250 = 0x02, /**< 250 kbit/s。 */
    BRT_BAUD_KBPS_125 = 0x03, /**< 125 kbit/s。 */
    BRT_BAUD_KBPS_100 = 0x04, /**< 100 kbit/s。 */
} brt_baud_rate_t;

/** 回传模式，枚举值等于线上参数。 */
typedef enum {
    BRT_REPORTING_QUERY = 0x00, /**< 查询。 */
    BRT_REPORTING_STANDARD_POSITION = 0xaa, /**< 标准帧自动回传位置。 */
    BRT_REPORTING_STANDARD_SIGNED_SPEED = 0x02, /**< 标准帧自动回传有符号速度。 */
    BRT_REPORTING_STANDARD_UNSIGNED_SPEED = 0x07, /**< 标准帧自动回传无符号速度。 */
    BRT_REPORTING_EXTENDED_POSITION = 0x18, /**< 扩展帧自动回传位置。 */
    BRT_REPORTING_EXTENDED_SIGNED_SPEED = 0x12, /**< 扩展帧自动回传有符号速度。 */
    BRT_REPORTING_EXTENDED_UNSIGNED_SPEED = 0x17, /**< 扩展帧自动回传无符号速度。 */
} brt_reporting_mode_t;

/** 计数递增方向，枚举值等于线上参数。 */
typedef enum {
    BRT_DIRECTION_CLOCKWISE = 0x00, /**< 顺时针。 */
    BRT_DIRECTION_COUNTERCLOCKWISE = 0x01, /**< 逆时针。 */
} brt_direction_t;

/** 主机请求的功能，枚举值等于线上功能码。 */
typedef enum {
    BRT_REQUEST_READ_POSITION = 0x01, /**< 读取位置。 */
    BRT_REQUEST_SET_STANDARD_ADDRESS = 0x02, /**< 设置标准地址。 */
    BRT_REQUEST_SET_EXTENDED_ADDRESS = 0x22, /**< 设置扩展地址。 */
    BRT_REQUEST_SET_BAUD_RATE = 0x03, /**< 设置波特率。 */
    BRT_REQUEST_SET_REPORTING_MODE = 0x04, /**< 设置回传模式。 */
    BRT_REQUEST_SET_REPORTING_PERIOD = 0x05, /**< 设置回传周期。 */
    BRT_REQUEST_SET_ZERO = 0x06, /**< 将当前位置设为零点。 */
    BRT_REQUEST_SET_DIRECTION = 0x07, /**< 设置计数递增方向。 */
    BRT_REQUEST_READ_SPEED = 0x0a, /**< 读取角速度。 */
    BRT_REQUEST_SET_SPEED_SAMPLE_TIME = 0x0b, /**< 设置速度采样时间。 */
    BRT_REQUEST_SET_MIDPOINT = 0x0c, /**< 将当前位置设为中点。 */
    BRT_REQUEST_SET_POSITION = 0x0d, /**< 设置当前位置计数。 */
    BRT_REQUEST_SET_FIVE_TURNS = 0x0f, /**< 将当前位置设为五圈值。 */
} brt_request_kind_t;

/** 主机请求的功能及其参数。例如，设置回传周期时写入 `value.reporting_period_us`。 */
typedef struct {
    brt_request_kind_t kind; /**< 请求类型，决定有效的参数成员。 */
    /** 无用户参数的请求由编码器生成协议固定字节。 */
    union {
        uint8_t standard_address; /**< 新标准地址，范围 1 到 255。 */
        uint32_t extended_address; /**< 新扩展地址，范围 0 到 0x1fffffff，线上大端。 */
        brt_baud_rate_t baud_rate; /**< 波特率枚举。 */
        brt_reporting_mode_t reporting_mode; /**< 回传模式枚举。 */
        uint16_t reporting_period_us; /**< 微秒，范围 50 到 65535，线上小端。 */
        brt_direction_t direction; /**< 计数递增方向枚举。 */
        uint16_t speed_sample_time_ms; /**< 毫秒，范围 0 到 65535，线上大端。 */
        uint32_t position; /**< 原始位置计数，线上大端。 */
    } value; /**< 参数。 */
} brt_request_t;

/** 设备应答的类型。 */
typedef enum {
    BRT_RESPONSE_POSITION, /**< 位置计数。 */
    BRT_RESPONSE_SPEED, /**< 有符号角速度原始值。 */
    BRT_RESPONSE_ACK, /**< 普通写入功能的一字节状态。 */
    BRT_RESPONSE_SET_EXTENDED_ADDRESS, /**< 扩展设址的四字节状态。 */
} brt_response_kind_t;

/**
 * 普通写入功能的应答。
 *
 * 读位置、读速度和扩展设址使用各自的应答类型。
 */
typedef struct {
    brt_request_kind_t command; /**< 已确认的普通写入功能。 */
    uint8_t status; /**< 原始状态，0 表示成功，其他值按原样保留。 */
} brt_ack_t;

/** 设备应答或主动回传的数据。例如，位置帧的原始计数取自 `value.position`。 */
typedef struct {
    brt_response_kind_t kind; /**< 应答类型，决定有效的数据成员。 */
    /** 按应答类型读取对应字段。 */
    union {
        uint32_t position; /**< 完整原始位置计数，线上小端。 */
        int32_t speed; /**< 完整有符号角速度原始值，线上小端。 */
        brt_ack_t ack; /**< 普通写入功能及其状态。 */
        uint32_t extended_address_status; /**< 扩展设址原始状态，线上大端，0 表示成功。 */
    } value; /**< 应答数据。 */
} brt_response_t;

/** 编解码结果。 */
typedef enum {
    BRT_OK, /**< 成功，输出有效。 */
    BRT_IGNORED, /**< 有效 CAN ID 的 Classic 数据帧与目标 ID 不匹配，或匹配帧通过长度和设备号校验后功能未知。 */
    BRT_ERROR_INVALID_ARGUMENT, /**< 必需的消息或输出指针为空，或 CAN ID 匹配、长度为 3 到 8 字节的 Classic 数据帧缺少数据指针。 */
    BRT_ERROR_INVALID_ID_FORMAT, /**< CAN ID 的格式枚举未定义。 */
    BRT_ERROR_INVALID_FRAME_KIND, /**< 帧形态枚举未定义。 */
    BRT_ERROR_INVALID_ADDRESS, /**< 目标标准 CAN ID 与八位设备号不同。 */
    BRT_ERROR_INVALID_REQUEST_KIND, /**< 请求类型枚举未定义。 */
    BRT_ERROR_INVALID_RESPONSE_KIND, /**< 应答类型枚举未定义。 */
    BRT_ERROR_INVALID_ACK_COMMAND, /**< 此功能未定义普通写入应答。 */
    BRT_ERROR_INVALID_STANDARD_CAN_ID, /**< CAN 标准标识符超过 11 位。 */
    BRT_ERROR_INVALID_EXTENDED_CAN_ID, /**< CAN 扩展标识符或新扩展地址超过 29 位。 */
    BRT_ERROR_UNSUPPORTED_REMOTE_FRAME, /**< 收到 RTR 帧。 */
    BRT_ERROR_UNSUPPORTED_CAN_FD, /**< 收到 CAN FD 帧。 */
    BRT_ERROR_DATA_LENGTH_EXCEEDS_CLASSIC_CAN, /**< Classic 帧有效字节数超过八。 */
    BRT_ERROR_INCOMPLETE_HEADER, /**< 有效字节数不足三个头部字段。 */
    BRT_ERROR_DECLARED_LENGTH_EXCEEDS_CLASSIC_CAN, /**< LEN 字段超过八。 */
    BRT_ERROR_LENGTH_MISMATCH, /**< LEN 与实际字节数不符。 */
    BRT_ERROR_DEVICE_ID_MISMATCH, /**< 匹配 CAN ID 的帧使用了不同的 DEVICE_ID。 */
    BRT_ERROR_INVALID_LENGTH, /**< 已知功能的报文长度不符合当前方向。 */
    BRT_ERROR_INVALID_FIXED_PARAMETER, /**< 固定请求参数在线上使用了其他值。 */
    BRT_ERROR_INVALID_STANDARD_ADDRESS, /**< 新标准地址为 0。 */
    BRT_ERROR_INVALID_BAUD_RATE, /**< 波特率参数未定义。 */
    BRT_ERROR_INVALID_REPORTING_MODE, /**< 回传模式参数未定义。 */
    BRT_ERROR_INVALID_DIRECTION, /**< 方向参数未定义。 */
    BRT_ERROR_INVALID_REPORTING_PERIOD, /**< 回传周期小于 50 微秒。 */
} brt_result_t;

/**
 * @brief 将主机请求编码为 Classic CAN 数据帧。
 * @param[in] address 显式目标地址。
 * @param[in] request 请求及对应的参数，须指向可读对象。
 * @param[out] out 须指向可写对象，仅成功时写入。
 * @return 成功返回 BRT_OK，地址、指针、请求类型或参数无效时返回对应错误。
 *
 * 0x22 请求总是编码为 LEN 7、DLC 7。
 */
brt_result_t brt_encode_request(brt_address_t address, const brt_request_t *request,
                                brt_encoded_frame_t *out);

/**
 * @brief 将接收帧按主机请求方向解码。
 * @param[in] address 显式匹配地址。
 * @param[in] frame 保留实际帧形态及长度的接收帧视图。
 * @param[out] out 须指向可写对象，仅成功时写入。
 * @retval BRT_OK 成功解出请求。
 * @retval BRT_IGNORED 有效 CAN ID 的 Classic 数据帧与目标 ID 不匹配，或匹配帧通过头部校验后功能未知。
 * @return 其他结果为具体错误。任意节点的 RTR 或 FD 帧均返回错误，目标 ID 匹配时设备号错误返回 BRT_ERROR_DEVICE_ID_MISMATCH。
 *
 * 先校验 CAN ID 位宽及帧形态，再过滤不匹配的 Classic 数据帧。匹配帧先校验数据长度、
 * LEN 和 DEVICE_ID，再检查功能及参数。0x22 请求的 LEN 可为 4 或 7，实际 DLC 必须为 7。
 */
brt_result_t brt_decode_request(brt_address_t address, brt_frame_ref_t frame, brt_request_t *out);

/**
 * @brief 将设备应答或主动回传编码为 Classic CAN 数据帧。
 * @param[in] address 应答所用地址，地址变化时由调用方指定。
 * @param[in] response 应答及对应的数据，须指向可读对象。
 * @param[out] out 须指向可写对象，仅成功时写入。
 * @return 成功返回 BRT_OK，地址、指针、应答类型或普通应答功能无效时返回对应错误。
 */
brt_result_t brt_encode_response(brt_address_t address, const brt_response_t *response,
                                 brt_encoded_frame_t *out);

/**
 * @brief 将接收帧按设备应答方向解码。
 * @param[in] address 显式匹配地址。
 * @param[in] frame 保留实际帧形态及长度的接收帧视图。
 * @param[out] out 须指向可写对象，仅成功时写入。
 * @retval BRT_OK 成功解出应答或主动回传。
 * @retval BRT_IGNORED 忽略条件与 brt_decode_request() 相同。
 * @return 其他结果为具体错误，包括不支持的帧形态和匹配 CAN ID 后发现的设备号错误。
 *
 * 帧过滤顺序见 brt_decode_request()。0x22 应答的 LEN 和 DLC 都须为 7。
 * 位置和有符号速度按功能码与字段解析，查询应答与主动回传共用布局。
 * 无符号速度回传布局未实现。
 */
brt_result_t brt_decode_response(brt_address_t address, brt_frame_ref_t frame, brt_response_t *out);

#endif
