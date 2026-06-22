# 版本升级到 0.1.1 — 代码审计报告

## 审计总结

| 维度 | 结论 |
|------|------|
| 方案符合性 | ✅ 版本来源已同步 |
| 红线约束 | ✅ 未修改业务逻辑、SQL、权限或接口 |
| 编码规范 | ✅ 配置和 UI 变更范围清晰 |
| 编译 / 测试 | ✅ 已运行 `cargo check --workspace --offline` 和前端构建 |
| 潜在风险 | 需后续单独提交并打 `v0.1.1` tag 才会触发正式 Release |

## 发现的问题

未发现阻断问题。

## 逐文件审计

### `Cargo.toml`

- 改动摘要：workspace package version 更新为 `0.1.1`。
- 规则检查：不涉及 Controller、SQL、DTO、金额计算等强制规则。
- 结论：通过。

### `Cargo.lock`

- 改动摘要：本项目 workspace 包 `rust_tool_cli`、`rust_tool_core`、`rust_tool_desktop`、`rust_tool_server` 更新为 `0.1.1`。
- 规则检查：未夹带第三方依赖版本升级。
- 结论：通过。

### `frontend/package.json`

- 改动摘要：前端 package version 更新为 `0.1.1`。
- 规则检查：不涉及前端业务逻辑或 API 调用。
- 结论：通过。

### `frontend/src-tauri/tauri.conf.json`

- 改动摘要：Tauri bundle version 更新为 `0.1.1`。
- 规则检查：配置变更与发布目标一致。
- 结论：通过。

### `frontend/vite.config.ts`

- 改动摘要：从 `frontend/package.json` 读取版本号，并通过 `define` 注入 `__APP_VERSION__`。
- 规则检查：版本展示与 package 版本同源，避免硬编码散落。
- 结论：通过。

### `frontend/src/vite-env.d.ts`

- 改动摘要：声明 `__APP_VERSION__` 全局常量类型。
- 规则检查：类型声明明确，构建期常量可被 Vue 组件安全使用。
- 结论：通过。

### `frontend/src/components/AppSidebar.vue`

- 改动摘要：在侧边栏顶部品牌区增加版本号展示；展开态显示在 `RustTool` 下方，折叠态显示在 `RT` 下方。
- 规则检查：未改变导航路由和菜单行为；文本尺寸和布局固定，不会挤压主工作区。
- 结论：通过。

## 红线合规性

| 红线 | 是否违反 | 说明 |
|------|----------|------|
| ARCH-001 | 否 | 未涉及 Controller / Service |
| SEC-002 | 否 | 未涉及 SQL |
| VALID-003 | 否 | 未涉及请求体 DTO |
| FINANCE-001 | 否 | 未涉及 BigDecimal |
| CLEAN-003 | 否 | 未涉及 Java FQCN |

## 验证记录

- 已运行：
  - `cargo pkgid --offline -p rust_tool_core`
  - `cargo pkgid --offline -p rust_tool_desktop`
  - `cargo pkgid --offline -p rust_tool_cli`
  - `cargo pkgid --offline -p rust_tool_server`
  - `node -e ...`
  - `git diff --check -- Cargo.toml Cargo.lock frontend/package.json frontend/src-tauri/tauri.conf.json .agents/tasks/260622_version_0_1_1`
  - `cargo check --workspace --offline`
  - `CI=true pnpm --dir frontend install --frozen-lockfile --ignore-scripts`
  - `pnpm --dir frontend build`
  - 本地浏览器视觉验证：确认侧边栏顶部品牌区显示 `v0.1.1`

## 结论

本次版本升级和版本显示变更可提交。正式发布仍需创建并推送 `v0.1.1` tag。
