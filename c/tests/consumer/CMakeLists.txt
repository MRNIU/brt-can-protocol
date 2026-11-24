# Copyright The odrive-can-driver Contributors
# Copyright The brt-can-protocol Contributors
# 以外部父工程方式消费协议库，供宿主运行和裸机交叉链接使用。

cmake_minimum_required(VERSION 3.21)
project(brt_can_consumer LANGUAGES C)

include(CTest)
add_subdirectory("${CMAKE_CURRENT_LIST_DIR}/../../.." brt_can_protocol)

add_executable(brt_can_consumer main.c)
target_link_libraries(brt_can_consumer PRIVATE brt_can_protocol::brt_can_protocol)
set_target_properties(brt_can_consumer PROPERTIES
    C_STANDARD 17
    C_STANDARD_REQUIRED YES
    C_EXTENSIONS NO)

if(BUILD_TESTING AND NOT CMAKE_CROSSCOMPILING)
    add_test(NAME brt_can_consumer COMMAND brt_can_consumer)
endif()
