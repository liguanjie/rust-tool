# 版本升级到 0.1.1 — 走查报告

## 变更概览

- 将 Rust workspace 版本从 `0.1.0` 升级为 `0.1.1`。
- 将 Tauri 打包配置版本从 `0.1.0` 升级为 `0.1.1`。
- 将前端 package 版本从 `0.1.0` 升级为 `0.1.1`。
- 将 `Cargo.lock` 中本项目 workspace 包版本同步为 `0.1.1`，未夹带第三方依赖升级。
- 在应用左侧顶部品牌区增加版本号展示，位于 `RustTool` 下方；折叠态显示在 `RT` 下方。

## 关键文件

- `Cargo.toml`
- `Cargo.lock`
- `frontend/package.json`
- `frontend/src-tauri/tauri.conf.json`
- `frontend/src/components/AppSidebar.vue`
- `frontend/src/vite-env.d.ts`
- `frontend/vite.config.ts`

## 验证结果

| 验证项 | 结果 | 说明 |
|--------|------|------|
| `cargo pkgid --offline -p rust_tool_core` | 通过 | 显示 `#0.1.1` |
| `cargo pkgid --offline -p rust_tool_desktop` | 通过 | 显示 `rust_tool_desktop@0.1.1` |
| `cargo pkgid --offline -p rust_tool_cli` | 通过 | 显示 `#0.1.1` |
| `cargo pkgid --offline -p rust_tool_server` | 通过 | 显示 `#0.1.1` |
| `node -e ... package/tauri version` | 通过 | 输出 `0.1.1 0.1.1` |
| `git diff --check -- ...` | 通过 | 无输出 |
| `cargo check --workspace --offline` | 通过 | workspace 编译检查通过 |
| `CI=true pnpm --dir frontend install --frozen-lockfile --ignore-scripts` | 通过 | lockfile 一致，依赖目录已恢复 |
| `pnpm --dir frontend build` | 通过 | 前端生产构建通过 |
| 本地浏览器视觉验证 | 通过 | 顶部品牌区显示版本号，底部仅保留程序配置 |

## 风险与注意事项

- 本次仅升级版本号，不创建 Git tag，也不触发正式 Release。
- 当前工作区存在其他未提交改动，本次版本升级和顶部版本显示未修改这些文件。

## 待用户验证

- 如需正式发布，提交本次版本升级后创建并推送 `v0.1.1` tag。
