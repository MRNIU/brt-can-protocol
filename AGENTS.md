<!-- Copyright The brt-can-protocol Contributors -->
<!-- 约定协议实现、文档、验证和发布的维护边界。 -->

# 本仓库开发约束

- 保持单个 library crate、Rust stable、Edition 2024；默认与 `embedded-can` 的 MSRV 为 Rust 1.85，必须实际验证。
- 默认 `no_std`、无 `alloc`、零运行依赖，禁止 `unsafe`；可选帧转换依赖与芯片选择不得泄漏进默认库。
- 协议模块按职责命名；说明书来源、协议依据与勘误入口集中在 README。
- 库只转换协议值与 CAN 帧，不添加外设访问、收发、任务、时序、重试、状态机、标定、物理换算、限幅或控制许可。
- 保留标准／扩展 ID、实际 RTR／FD 形态、未知状态码与完整数据位；不能把 FD 丢弃标志后送入 Classic 入口。
- 扩展 CAN ID 与 DEVICE_ID 保持显式的独立字段，库不自动派生或校验二者关系；调用方负责地址变化后的匹配、广播处理和设备操作结果。
- 测试使用独立给定协议字节，正文与示例冲突时按 README 已声明规则验证。
- 文档、rustdoc、提交说明使用中文，代码标识符使用英文。公开 API 应说明前提、单位、错误和返回语义。
- 每个手写文件使用合法格式的版权注释并简述职责；新增文件使用 `Copyright The brt-can-protocol Contributors`。复用的实质代码保留原版权，保留 LICENSE 原条款与归属。
- 文档和注释面向库的使用与维护，记录协议、API、示例和贡献规则；适配依赖与接入片段归 `examples/README.md`。
- 变更后执行 CONTRIBUTING 与 CI 的相关检查，并报告实际执行结果。
- 提交精确暂存本任务改动，采用中文 Conventional Commit、DCO signoff 和适用的 AI co-author；发布、push、tag、Release 需要明确授权。
