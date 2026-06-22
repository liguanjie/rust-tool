# GitHub Actions pnpm 构建脚本白名单修复 — 代码审计报告

## 审计总结

| 维度 | 结论 |
|------|------|
| 方案符合性 | ✅ 符合：精准修复 pnpm 供应链策略阻断 |
| 红线约束 | ✅ 未放开全部脚本，未修改无关打包配置 |
| 编码规范 | ✅ 配置改动简洁明确 |
| 编译 / 测试 | ✅ 依赖安装与前端构建已通过 |
| 潜在风险 | 后续进入 Tauri 打包阶段后可能出现新的平台打包问题 |

## 发现的问题

未发现阻断问题。

## 逐文件审计

### `frontend/pnpm-workspace.yaml`

- 改动摘要：将 `allowBuilds.vue-demi` 从未决占位值改为 `true`。
- 规则检查：
  - 未使用全局放开脚本的配置。
  - 保留 `core-js: false`、`esbuild: true` 既有策略。
  - 改动范围仅限前端依赖安装配置。
- 结论：通过。

## 红线合规性

| 红线 | 是否违反 | 说明 |
|------|----------|------|
| 不放开所有依赖的构建脚本 | 否 | 仅允许 `vue-demi` |
| 不修改 GitHub Actions 权限、Release 配置或 Tauri 打包参数 | 否 | 未改 workflow |
| 不调整不相关依赖版本 | 否 | 未改 lockfile 与 package 版本 |

## 边界场景推演

| 场景 | 结论 | 说明 |
|------|------|------|
| CI 依赖安装 | 通过 | `vue-demi` 构建脚本已明确批准 |
| 前端构建 | 通过 | `pnpm --dir frontend run build` 成功 |
| 后续新增构建脚本依赖 | 需再次确认 | pnpm 会继续按白名单策略拦截未批准依赖 |

## 验证记录

- 已运行：
  - `pnpm --dir frontend install --frozen-lockfile`
  - `pnpm --dir frontend run build`
  - `git diff --check -- frontend/pnpm-workspace.yaml .agents/tasks/260622_github_actions_pnpm_builds`
- 未运行：
  - 远程 Release workflow 重新运行。原因：修复尚未提交到远程分支。

## 结论

本次配置修复可提交远程后重新触发 Release Tauri App 工作流。
