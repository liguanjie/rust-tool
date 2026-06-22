# 数据库维护能力 — 走查报告

## 变更概览

- 程序配置页增加数据库大小卡片和文件大小明细。
- 程序配置页增加刷新状态、打开目录、备份数据库、清理历史四个维护动作。
- Core storage 增加数据库大小统计、备份和 OSV 命令历史清理。
- Tauri 增加数据库维护命令，并将结果回填到配置页状态。

## 关键文件

- `crates/rust_tool_core/src/storage.rs`
- `crates/rust_tool_core/src/lib.rs`
- `frontend/src-tauri/src/lib.rs`
- `frontend/src/api/programSettings.ts`
- `frontend/src/pages/ProgramSettings.vue`
- `frontend/src/api/programSettings.test.ts`

## 核心流程

1. 配置页加载时调用 `getProgramSettings`。
2. Tauri 读取配置路径，初始化/检查 SQLite，并返回 health 与 file stats。
3. 备份数据库时，Tauri 生成唯一备份路径，core storage 执行 `VACUUM INTO`。
4. 清理历史时，Tauri 清理 Agent 执行历史和 OSV 命令历史，并同步清空旧 JSON 快照中的 OSV command history。

## 验证结果

- `cargo test -p rust_tool_core storage`：通过，12 个 storage 测试全部成功。
- `cargo check -p rust_tool_desktop`：通过。
- `pnpm --dir frontend test:run`：通过，40 个前端测试全部成功。
- `pnpm --dir frontend build`：通过；仍有既有 Rollup PURE 注释和大 chunk 提示。
- `git diff --check`：通过。
- Browser UI 验证：桌面宽度和 390px 窄屏均无水平溢出，维护按钮可见，Web 模式下桌面维护按钮禁用，控制台无 error。

## 风险与注意事项

- 当前“清理历史”不会压缩数据库文件。若用户需要回收磁盘空间，下一步可以增加显式压缩数据库动作。
- “打开目录”在 Linux 上依赖 `xdg-open`，如果用户系统缺失该命令会返回错误提示。

## 待用户验证

- 在桌面 App 中打开“程序配置”，确认数据库大小展示正常。
- 点击“备份数据库”，确认生成备份文件。
- 点击“打开目录”，确认系统文件管理器打开数据库目录。
- 点击“清理历史”，确认 Agent 历史和 OSV 命令历史清空，项目清单和最后一次扫描结果保留。
