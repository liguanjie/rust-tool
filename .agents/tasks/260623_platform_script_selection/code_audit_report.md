# 平台脚本选择 — 代码审计报告

## 审计总结

| 维度 | 结论 |
|------|------|
| 方案符合性 | ✅ 核心逻辑下沉在 `rust_tool_core`，Tauri 与 Web 共用 |
| 红线约束 | ✅ 未发现本次改动违反红线 |
| 编码规范 | ✅ 本次改动文件未发现阻断问题 |
| 编译 / 测试 | ✅ `cargo test`、`pnpm --dir frontend run build` 通过 |
| 潜在风险 | Windows 实机执行未在当前 macOS 环境验证；clippy 存在既有阻断 |

## 发现的问题

未发现本次改动引入的阻断问题。

既有阻断：

- `crates/rust_tool_core/src/tools/finalshell_password.rs:149`：`cargo clippy --all-targets -- -D warnings` 报 `manual_is_multiple_of`。该文件不属于本次改动范围，未在本任务中修改。

## 逐文件审计

### `crates/rust_tool_core/src/workbench.rs`

- 改动摘要：新增平台脚本扩展判断、按平台构建脚本执行器、执行前扩展校验、平台过滤单测。
- 规则检查：
  - `ARCH-001`：通过，脚本发现与执行策略在核心层统一实现。
  - `EX-001`：通过，运行时错误通过 `Result` 返回；新增测试中的断言仅用于单测。
  - `CLEAN-001`：本文件通过 clippy 检查；全量 clippy 被既有文件阻断。
- 结论：通过。

### `frontend/src/pages/AgentSkills.vue`

- 改动摘要：新增安装脚本识别函数，批量安装合并逻辑兼容 `.sh` 与 `.ps1`。
- 规则检查：
  - `ARCH-002`：通过，前端仍调用统一 Tauri / HTTP 适配路径。
  - `VUE-002`：通过，新增轻量辅助函数，没有引入不必要状态或抽象。
- 结论：通过。

## 红线合规性

| 红线 | 是否违反 | 说明 |
|------|----------|------|
| 核心业务不得散落在 Tauri / server 层 | 否 | 入口层未改动，逻辑集中在 `rust_tool_core` |
| 不改变安装参数格式 | 否 | 仍使用 `项目路径 项目类型` |
| 不执行非当前平台脚本 | 否 | 执行前校验扩展，不匹配时返回错误 |

## 边界场景推演

| 场景 | 结论 | 说明 |
|------|------|------|
| 正向场景 | 通过 | macOS API 返回 `.sh`，UI 展示两个 `.sh` 安装模块 |
| 回归场景 | 通过 | 批量安装向导仍合并为单个入口 |
| NULL / 空数据 | 通过 | 无扩展或不支持扩展会返回明确错误 |
| 并发 / 状态变化 | 无新增风险 | 未改变前端运行状态机和执行循环 |

## 性能影响

- 调用频率：仅脚本目录扫描和用户手动执行时触发。
- 数据量级：本地脚本文件数量，影响极低。
- 索引 / JOIN 影响：不涉及数据库。

## 验证记录

- 已运行：
  - `cargo fmt`
  - `git diff --check`
  - `cargo test -p rust_tool_core workbench`
  - `cargo test`
  - `pnpm --dir frontend run build`
  - `cargo clippy --all-targets -- -D warnings`
  - 本机 API 验证 `/api/workbench/scripts`
  - in-app Browser 验证 `/agent-skills`
- 未通过：
  - `cargo clippy --all-targets -- -D warnings`，失败原因是既有 `finalshell_password.rs:149` clippy 提示。
- 未运行：
  - Windows 实机 PowerShell 执行验证。

## 结论

本次平台脚本选择改动符合项目同源架构要求，macOS 当前平台验证通过；建议后续在 Windows 环境补做一次真实安装执行验证。
