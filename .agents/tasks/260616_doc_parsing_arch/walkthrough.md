# 办公文档智能解析架构升级 — 走查报告

## 变更概览

本次将原架构设计文档从“四层混合分流草案”升级为生产级 `Document AI Pipeline` 架构。核心变化是把 Markdown 从主数据模型降级为派生视图，新增 `Canonical Document JSON` 作为唯一可信中间层，并补充页级/区域级动态路由、结构融合、质量控制、人审、评测和可观测闭环。

## 关键文件

| 文件 | 说明 |
|------|------|
| `architecture_design.md` | 主架构文档，已重写为生产级方案 |
| `solution_proposal.md` | 供应链附件解析落地方案，明确第一版工具组合、部署形态、路由规则和验收指标 |
| `implementation_plan.md` | 本次文档升级计划与验证记录 |
| `task.md` | 任务执行清单 |
| `walkthrough.md` | 本走查报告 |

## 核心流程

更新后的架构主线如下：

```text
文件接入
 -> 安全网关
 -> 文档画像
 -> 页级/区域级路由
 -> 多解析器抽取
 -> 结构融合
 -> Canonical Document JSON
 -> Markdown / Table Store / Retrieval Index 派生视图
 -> LLM Agent 与工具调用
 -> 业务输出与审计闭环
```

## 重点设计调整

- 从文档级双轨分流升级为页级/区域级动态路由。
- 从 Markdown 主导升级为 `Canonical Document JSON` 主导。
- 明确 MarkItDown、PaddleOCR、PDF text layer、Layout Parser、Table Parser、Pandas/DuckDB、VLM 的职责边界。
- 新增供应链落地方案：开源主链路、人工兜底、人工复核闭环、MVP 单机部署。
- 根据最新决策移除商业 IDP 兜底口径，低置信和关键冲突结果改为人工复核 / 人工录入处理。
- 补充 Word、PDF、Excel 内嵌图片处理：容器文件解析时抽取图片资产，记录来源位置，并进入 OCR / 人工复核链路。
- 补充 Word、PDF、Excel 内嵌表格和复杂格式处理：抽取嵌套表格、合并单元格、文本框、批注、脚注、公式、隐藏行列、透视表等结构，并进入 Table Parser / Structure Normalizer。
- 补充解析参数控制原则：接口参数、模板配置和系统配置负责底层解析行为；LLM 只负责推理、校验、解释和提出重跑建议。
- 补充软件安装准备清单：明确 Python 依赖、PaddleOCR 扩展、OpenCV headless、LibreOffice、PostgreSQL、Redis、对象存储、Tesseract 等安装决策。
- 根据 PaddleOCR 官方中文 README 校准选型：`PP-OCRv6` 作为默认 OCR，`PP-StructureV3` 作为坐标级结构解析，`PaddleOCR-VL-1.6` 作为高复杂文档理解增强能力。
- 表格独立进入 Table Store，避免大表直接塞入 LLM 上下文。
- RAG 切块基于语义元素，保留 `orig_block_ids` 以支持引用溯源。
- LLM Agent 通过表格查询、原文定位、检索、规则校验等工具完成业务推理。
- 增加置信度阈值、人审队列、评测集、可观测性和安全合规章节。

## 验证结果

| 验证项 | 结果 |
|--------|------|
| 架构文档可读性检查 | 通过 |
| 原组件定位保留 | 通过 |
| 生产级闭环覆盖 | 通过 |
| 供应链落地方案补充 | 通过 |
| 任务目录文件清单检查 | 通过 |

## 风险与注意事项

- 当前仍是架构文档升级，尚未通过真实样本文档 POC 验证组件效果。
- 具体解析器选型需要后续按样本集比较准确率、耗时、内存和部署成本。
- `Canonical Document JSON` 的字段需要在进入实现阶段前再细化为稳定 schema。
- 如果处理涉密文件，外部 LLM/VLM 调用必须增加授权、脱敏和审计策略。

## 待用户验证

- 确认当前架构方向是否作为后续实现基线。
- 确认 MVP 阶段优先支持的文件类型：PDF、DOCX、XLSX、图片/扫描件的优先级。
- 确认是否需要进一步补充 POC 方案、数据模型 schema 或模块拆分任务。
