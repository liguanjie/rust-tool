# OSV 项目清单与命令历史 SQLite 存储实施记录

## 变更内容

- 新增 `osv_projects` 表保存 OSV 项目清单，使用 `sort_order` 保持页面顺序。
- 新增 `osv_command_history` 表保存轻量命令历史，`record_json` 保留完整命令记录，拆出 `kind`、`project_path`、`started_at` 便于后续查询。
- Core storage 新增 `list_osv_projects`、`replace_osv_projects`、`list_osv_command_history`、`replace_osv_command_history`。
- Tauri `get_osv_settings` 首次读取会把旧 JSON 中的项目/历史导入 SQLite。
- Tauri `save_osv_settings` 改为将项目/历史写入 SQLite，同时把组合后的 settings 写回 JSON 快照。

## 验证

- `cargo test -p rust_tool_core storage`：通过，9 个 storage 测试全部成功。
- `cargo check -p rust_tool_desktop`：通过。
- `pnpm --dir frontend test:run`：通过，38 个前端测试全部成功。
- `pnpm --dir frontend build`：通过；仍有既有 Rollup PURE 注释和大 chunk 提示。
- `git diff --check`：通过。

## 待用户验证

- 桌面 App 中打开 OSV Scanner，确认已有项目列表能正常出现。
- 新增/删除项目后重启 App，确认项目列表保持一致。
- 执行几次扫描/导出后重启 App，确认命令历史保留最近记录。

## 备注

- 本次没有改 OSV 页面视觉结构，因此未做额外浏览器视觉回归。
