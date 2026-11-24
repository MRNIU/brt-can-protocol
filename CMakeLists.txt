# Copyright The odrive-can-driver Contributors
# Copyright The brt-can-protocol Contributors
# 提供独立或嵌入父工程的 C17 静态库，并注册独立宿主测试。

cmake_minimum_required(VERSION 3.21)
project(brt_can_protocol LANGUAGES C)

add_library(brt_can_protocol STATIC c/src/protocol.c)
add_library(brt_can_protocol::brt_can_protocol ALIAS brt_can_protocol)
target_include_directories(brt_can_protocol PUBLIC "${CMAKE_CURRENT_SOURCE_DIR}/c/include")
target_compile_features(brt_can_protocol PUBLIC c_std_17)
set_target_properties(brt_can_protocol PROPERTIES
    C_STANDARD 17
    C_STANDARD_REQUIRED YES
    C_EXTENSIONS NO)

if(PROJECT_IS_TOP_LEVEL AND NOT CMAKE_CROSSCOMPILING)
    include(CTest)
    if(BUILD_TESTING)
        add_executable(brt_can_test_protocol c/tests/test_protocol.c)
        target_link_libraries(brt_can_test_protocol PRIVATE brt_can_protocol::brt_can_protocol)
        set_target_properties(brt_can_test_protocol PROPERTIES
            C_STANDARD 17
            C_STANDARD_REQUIRED YES
            C_EXTENSIONS NO)
        add_test(NAME brt_can_test_protocol
            COMMAND brt_can_test_protocol "${CMAKE_CURRENT_SOURCE_DIR}/tests/fixtures/brt_v2_7.tsv")
    endif()
endif()
