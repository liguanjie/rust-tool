# GitHub Actions pnpm 构建脚本白名单修复 — 任务清单

## 阶段1：定位与设计

- [x] 使用 GitHub 插件读取失败 job
- [x] 定位失败步骤为 `Install frontend dependencies`
- [x] 确认错误为 `ERR_PNPM_IGNORED_BUILDS`
- [x] 确认最小修复方案

## 阶段2：实现

- [x] 更新 pnpm 构建脚本白名单

## 阶段3：验证与收尾

- [x] 依赖安装验证
- [x] 前端构建验证
- [x] 走查报告
- [x] 代码审计报告
