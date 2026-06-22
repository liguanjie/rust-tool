# master-latest Release 策略修复 — 代码审计报告

## 审计总结

| 维度 | 结论 |
|------|------|
| 方案符合性 | ✅ 手动构建和正式版本发布已分流 |
| 红线约束 | ✅ 未修改应用版本号，未混入无关代码 |
| 编码规范 | ✅ workflow 条件清晰，文档同步更新 |
| 编译 / 测试 | GitHub Actions `Release Tauri App #5` 已通过 |
| 潜在风险 | `master-latest` 是移动 tag，不能作为正式归档 |

## 发现的问题

未发现阻断问题。

## 逐文件审计

### `.github/workflows/publish.yml`

- 改动摘要：新增 `prepare-master-latest` job，手动触发时重建 `master-latest` 预发布；将 Tauri 打包拆成 `workflow_dispatch` 的 master 预览发布和 `push` tag 的正式版本发布。
- 规则检查：
  - 未改变权限范围，仍使用 `contents: write`。
  - 未放大触发范围，仍只支持 `v*` tag push 和手动触发。
  - `workflow_dispatch` 和 `push` 两个打包步骤互斥。
- 结论：通过。

### `github_ops_guide.md`

- 改动摘要：补充 `master-latest` 预发布 Release 说明，并新增正式版本发布说明。
- 规则检查：文档与 workflow 行为一致。
- 结论：通过。

## 红线合规性

| 红线 | 是否违反 | 说明 |
|------|----------|------|
| 不修改 Tauri 应用版本号 | 否 | 未改 `tauri.conf.json` |
| 不把正式版本 Release 当作 master 测试包归档 | 否 | 手动触发改用 `master-latest` |
| 不混入当前工作区中不相关 Rust 改动 | 否 | 本次提交只计划包含 workflow 和指南 |

## 边界场景推演

| 场景 | 结论 | 说明 |
|------|------|------|
| 手动触发 master 构建 | 通过 | tag 移动到 `GITHUB_SHA` 后上传到 `master-latest` |
| 正式 tag 构建 | 通过 | `github.ref_name` 对应 `v*` tag |
| 重复手动触发 | 通过 | `master-latest` Release 会被重建，tag 会被强制移动 |
| 已存在正式版本 Release | 不影响 | 手动触发不再使用 `v__VERSION__` |

## 性能影响

- 调用频率：仅 GitHub Actions 发布时执行。
- 数据量级：不涉及业务数据。
- 索引 / JOIN 影响：不涉及。

## 验证记录

- 已运行：
  - `git diff --check -- .github/workflows/publish.yml github_ops_guide.md .agents/tasks/260622_master_latest_release`
  - `git push origin master`
  - GitHub Actions `Release Tauri App #5`
  - `master-latest` Release 页面资产与提交核验

## 结论

本次 workflow 策略修复已提交并推送，远程手动触发验证通过。
