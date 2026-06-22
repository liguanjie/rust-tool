# 版本升级到 0.1.1 — 实施计划

## 需求与决策

- 需求描述：将 RustTool 应用版本从 `0.1.0` 升级到 `0.1.1`。
- 设计决策：同步更新 Rust workspace 版本、Tauri 打包版本、前端 package 版本，并让锁文件保持一致。
- 用户确认项：无，用户已明确指定目标版本 `0.1.1`。

## 改动清单

| # | 文件 | 操作 | 改动说明 |
|---|------|------|----------|
| 1 | `Cargo.toml` | MODIFY | 更新 workspace package version |
| 2 | `Cargo.lock` | MODIFY | 更新 workspace crate lock 版本 |
| 3 | `frontend/package.json` | MODIFY | 更新前端 package version |
| 4 | `frontend/src-tauri/tauri.conf.json` | MODIFY | 更新 Tauri bundle version |

## 质量保障

| 类型 | 命令 / 方法 | 预期 |
|------|-------------|------|
| 代码检查 | `git diff --check` | 无输出 |
| 锁文件同步 | `cargo generate-lockfile` | 成功 |
| 前端锁文件检查 | `pnpm --dir frontend install --lockfile-only --ignore-scripts` | 成功或确认无需变更 |

## 风险与回滚

- 风险：版本号未同步会导致安装包文件名或 Cargo lock 仍显示旧版本。
- 回滚：将上述文件版本号恢复为 `0.1.0`，重新生成锁文件。
