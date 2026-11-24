# Copyright The odrive-can-driver Contributors
# Copyright The brt-can-protocol Contributors
# 为最小消费者提供 Cortex-M7 硬浮点工具链，验证静态库可链接到裸机程序。

set(CMAKE_SYSTEM_NAME Generic)
set(CMAKE_SYSTEM_PROCESSOR arm)
find_program(CMAKE_C_COMPILER arm-none-eabi-gcc REQUIRED)
set(CMAKE_TRY_COMPILE_TARGET_TYPE STATIC_LIBRARY)

set(CMAKE_C_FLAGS_INIT "-mcpu=cortex-m7 -mthumb -mfpu=fpv5-d16 -mfloat-abi=hard")
set(CMAKE_EXE_LINKER_FLAGS_INIT "--specs=nosys.specs")
