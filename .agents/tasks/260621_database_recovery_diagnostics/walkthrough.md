# 数据库压缩、恢复与诊断 — 走查报告

## 变更概览

- 增加数据库压缩：执行 WAL checkpoint + `VACUUM`。
- 增加从备份恢复：恢复前自动备份当前库，关闭连接后替换数据库文件并重新初始化检查。
- 增加旧库清理：仅删除 `rusttool.sqlite` 及 sidecar 文件，不做迁移。
- 增加存储诊断：展示 Agent 历史、OSV 项目、OSV 命令历史、OSV 最新扫描结果、工具配置、导出记录、应用事件的记录数。
- 程序配置页新增维护按钮和诊断卡片。

## 关键文件

- `crates/rust_tool_core/src/storage.rs`
- `crates/rust_tool_core/src/lib.rs`
- `frontend/src-tauri/src/lib.rs`
- `frontend/src/api/programSettings.ts`
- `frontend/src/pages/ProgramSettings.vue`
- `frontend/src/api/programSettings.test.ts`

## 核心流程

1. 配置页加载 `getProgramSettings`。
2. Tauri 返回健康状态、文件大小、诊断计数和旧库状态。
3. 压缩数据库调用 `compact_program_database`，由 core 执行 `VACUUM`。
4. 从备份恢复调用 `restore_program_database`，先生成恢复前备份，再替换当前数据库。
5. 清理旧库调用 `delete_legacy_program_database`，只删除固定旧库文件。

## 验证结果

- `cargo test -p rust_tool_core storage`：通过，15 个 storage 测试全部成功。
- `cargo check -p rust_tool_desktop`：通过。
- `pnpm --dir frontend test:run`：通过，40 个前端测试全部成功。
- `pnpm --dir frontend build`：通过；仍有既有 Rollup PURE 注释和大 chunk 提示。
- `git diff --check`：通过。
- Browser UI 验证：桌面宽度和 390px 窄屏均无水平溢出；新增按钮与存储诊断可见；Web 模式下本地维护动作禁用；控制台无 error。
- 已按用户确认删除本机旧库 `~/Library/Application Support/com.ben.rusttool/rusttool.sqlite` 及 sidecar 文件；目录中当前保留 `rusttool.db`、`rusttool.db-wal`、`rusttool.db-shm` 和 `rusttool-settings.json`。

## 风险与注意事项

- 恢复备份是破坏性替换动作，已通过前端确认和恢复前自动备份降低风险。
- Browser 验证不覆盖真实 Tauri 文件选择对话框，需要桌面端实际点一次“从备份恢复”和“清理旧库”确认系统行为。
- 当前诊断展示记录数，不展示单表体积；SQLite 单表体积需要额外 dbstat 支持，暂不引入。

## 待用户验证

- 在桌面 App 点击“压缩数据库”，确认状态刷新正常。
- 点击“从备份恢复”，选择一个备份文件，确认恢复前备份路径提示正常。
- 若仍存在 `rusttool.sqlite`，点击“清理旧库”，确认 Finder 中旧库消失。
- 检查“存储诊断”记录数是否符合预期。
