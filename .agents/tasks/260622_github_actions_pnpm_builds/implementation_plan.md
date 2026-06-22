# GitHub Actions pnpm 构建脚本白名单修复 — 实施计划

## 需求与决策

- 需求描述：Release Tauri App 工作流在 `Install frontend dependencies` 步骤失败。
- 失败原因：GitHub Actions 日志显示 `[ERR_PNPM_IGNORED_BUILDS] Ignored build scripts: vue-demi@0.14.10`。
- 设计决策：将 `vue-demi` 加入 pnpm 允许构建脚本清单，保持现有 `core-js: false` 和 `esbuild: true` 策略不变。

## 系统现状分析

| # | 现状 | 位置 | 影响 |
|---|------|------|------|
| 1 | `vue-demi` 为未决占位 `set this to true or false` | `frontend/pnpm-workspace.yaml` | CI 环境 pnpm 11 安装依赖后因未批准构建脚本退出 |
| 2 | `.npmrc` 设置 `ignore-scripts=true` | `frontend/.npmrc` | 依赖脚本由 pnpm 供应链策略显式控制 |

## 改动清单

| # | 文件 | 操作 | 改动说明 |
|---|------|------|----------|
| 1 | `frontend/pnpm-workspace.yaml` | MODIFY | 将 `vue-demi` 构建脚本批准状态改为 `true` |

## 红线约束

1. 不放开所有依赖的构建脚本。
2. 不修改 GitHub Actions 权限、Release 配置或 Tauri 打包参数。
3. 不调整不相关依赖版本。

## 质量保障

| 类型 | 命令 / 方法 | 预期 |
|------|-------------|------|
| 配置检查 | `pnpm --dir frontend install --frozen-lockfile --force` | 不再出现 `ERR_PNPM_IGNORED_BUILDS` |
| 前端构建 | `pnpm --dir frontend run build` | 通过 |

## 风险与回滚

- 风险：后续新增有构建脚本的依赖仍需单独审批。
- 回滚：将 `vue-demi` 恢复为未批准或 `false`，但 Release 工作流会继续失败。
