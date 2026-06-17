# OSV 本地漏洞扫描管理器 - Task List

本任务列表基于 `requirements_background.md` 与 `implementation_plan.md`。执行顺序以 MVP 手动扫描闭环优先，第二阶段能力单独排期。

---

## M0 预研与边界确认

- [x] OSV-0001 确认本机 `osv-scanner` 版本与能力
  - 记录 `osv-scanner --version`
  - 记录 `osv-scanner scan source --help`
  - 确认 `--format json`、`--format html`、`--output-file`、`--config`、`--lockfile`、`--recursive` 参数行为
  - 验收：形成可实现的参数 allowlist 草案

- [x] OSV-0002 准备 JSON 输出样例
  - 覆盖无漏洞、有漏洞、含 Critical/High、含 ignored 或 config 影响的场景
  - 尽量覆盖 Rust、Node、Go 至少两类项目
  - 验收：样例可用于 core 单元测试

- [x] OSV-0003 确认报告导出目录策略
  - 桌面端默认候选：Downloads
  - Web 端默认候选：`/private/tmp`，用户仍可输入服务端可访问的绝对路径
  - 验收：方案明确，不覆盖源码、锁文件、配置文件

---

## M1 Rust Core 基础模型

- [x] OSV-0101 新增 core 模块
  - 新增 `crates/rust_tool_core/src/tools/osv_scanner.rs`
  - 修改 `crates/rust_tool_core/src/tools/mod.rs`
  - 修改 `crates/rust_tool_core/src/lib.rs`
  - 验收：模块可被 server 和 Tauri 同时引用

- [x] OSV-0102 定义核心错误类型
  - 使用 `thiserror` 定义 `OsvScannerError`
  - 覆盖未安装、路径非法、命令拒绝、扫描失败、JSON 解析失败、导出失败、忽略规则更新失败
  - 验收：核心接口不以 `String` 作为主要错误类型

- [x] OSV-0103 定义核心数据结构
  - `OsvInstallStatus`
  - `OsvCommandPreview`
  - `OsvCommandExecutionRecord`
  - `OsvScanCommandRequest`
  - `OsvScanRequest`
  - `OsvScanResult`
  - `OsvReportFormat`
  - `OsvReportExportCommandRequest`
  - `OsvReportExportRequest`
  - `OsvReportExportResult`
  - `OsvIgnoreResult`
  - 验收：所有前后端共享结构支持 `serde` 序列化

- [x] OSV-0104 实现路径校验
  - 对项目路径执行 `canonicalize`
  - 校验目标为目录
  - 限制空路径、过长路径、明显非法路径
  - 验收：非法路径返回稳定错误码，不触发命令执行

---

## M2 命令预览、校验与审计

- [x] OSV-0201 实现 `check_osv_scanner_installed`
  - 检测 PATH 中的 `osv-scanner`
  - 返回 binary path 和 version 信息
  - 验收：未安装和已安装路径均有稳定返回

- [x] OSV-0202 实现扫描命令预览
  - 实现 `build_scan_command`
  - 默认生成 `osv-scanner scan source --format json -r <project>`
  - 返回 `argv`、`display_command`、`working_dir`、`locked_args`、`editable_options`、`warnings`
  - 验收：UI 可在执行前展示完整命令

- [x] OSV-0203 实现命令草稿校验
  - 禁止替换二进制
  - 禁止删除 `scan source`
  - 应用内扫描必须锁定 `--format json`
  - 拒绝 shell 操作符、管道、重定向、命令替换、环境变量赋值
  - 仅允许 allowlist 中的高级参数
  - 验收：无法通过 API 绕过校验执行任意命令

- [x] OSV-0204 实现 shell-safe 展示命令
  - 基于 `argv` 生成仅用于展示和复制的命令字符串
  - 正确处理空格、引号和特殊字符
  - 验收：展示字符串不作为执行来源

- [x] OSV-0205 实现执行记录生成
  - 记录 id、kind、project_path、working_dir、argv、display_command
  - 记录 started_at、finished_at、duration_ms、exit_code、status
  - 保存 summary 与截断后的 stderr_excerpt
  - 验收：成功和失败均可追溯当时实际执行命令

---

## M3 扫描与结果解析

- [x] OSV-0301 实现 `scan_project`
  - 接收用户确认过的命令草稿
  - core 重新校验命令
  - 使用 `std::process::Command` 执行 `argv`
  - 不经过 shell
  - 验收：扫描结果包含结构化结果和执行记录

- [x] OSV-0302 实现 JSON 解析
  - 基于 OSV JSON 样例定义强类型结构
  - 保留未知字段兼容空间
  - 提取漏洞 ID、严重级别、包名、版本、影响路径、修复建议
  - 验收：样例 JSON 可稳定反序列化

- [x] OSV-0303 实现健康评分
  - 无漏洞为高分
  - Critical/High 明显拉低评分
  - 多漏洞组合有可解释扣分规则
  - 验收：评分测试覆盖无漏洞、High、Critical、多漏洞

- [x] OSV-0304 实现扫描摘要
  - 统计漏洞总数
  - 按严重程度聚合
  - 记录最高严重级别
  - 生成用户可读 summary
  - 验收：前端无需自行推导核心统计

---

## M4 报告导出 JSON/HTML

- [x] OSV-0401 实现导出命令预览
  - 实现 `build_export_command`
  - 支持 `OsvReportFormat::Json`
  - 支持 `OsvReportFormat::Html`
  - 使用 `--output-file <path>`
  - 验收：导出前可查看格式、路径和完整命令

- [x] OSV-0402 实现导出路径校验
  - JSON 导出限制 `.json`
  - HTML 导出限制 `.html`
  - 不允许覆盖源码文件、锁文件、`osv-scanner.toml`
  - 验收：未知格式和扩展名不匹配会被拒绝

- [x] OSV-0403 实现 `export_report`
  - core 重新校验导出命令
  - 执行 `osv-scanner scan source --format <json|html> --output-file <path>`
  - 返回 output_path、format、command record
  - 验收：JSON 和 HTML 文件实际生成，命令历史可追溯

- [x] OSV-0404 明确不启用 `--serve`
  - MVP 中不提供本地服务 HTML 报告
  - 文案说明 HTML 是静态文件导出
  - 验收：无端口占用和后台进程生命周期问题

---

## M5 忽略规则管理

- [x] OSV-0501 选择 TOML 编辑策略
  - 优先使用能保留原配置结构的 TOML 编辑库
  - 如新增依赖，更新 `crates/rust_tool_core/Cargo.toml`
  - 验收：不会用脆弱字符串拼接维护配置

- [x] OSV-0502 实现 `ignore_vulnerability`
  - 校验漏洞 ID
  - 校验忽略原因非空且长度合理
  - 写入或更新项目本地 `osv-scanner.toml`
  - 避免重复规则
  - 验收：重复忽略不会产生重复配置

- [x] OSV-0503 忽略规则测试
  - 新建配置文件
  - 追加到已有配置文件
  - 重复 ID 处理
  - 非法 ID 拒绝
  - 验收：生成 TOML 合法

---

## M6 Axum Web 入口

- [x] OSV-0601 新增 Web route 模块
  - 新增 `crates/rust_tool_server/src/routes/osv_scanner.rs`
  - 修改 `crates/rust_tool_server/src/routes/mod.rs`
  - 验收：路由模块编译通过

- [x] OSV-0602 注册 HTTP 路由
  - `GET /api/tools/osv-scanner/install-status`
  - `POST /api/tools/osv-scanner/scan/preview`
  - `POST /api/tools/osv-scanner/scan`
  - `POST /api/tools/osv-scanner/export/preview`
  - `POST /api/tools/osv-scanner/export`
  - `POST /api/tools/osv-scanner/ignore`
  - 验收：Web 版具备与桌面版等价的核心能力

- [x] OSV-0603 实现错误映射
  - 统一返回 `{ error: { code, message } }`
  - 映射 `osv_not_installed`
  - 映射 `invalid_project_path`
  - 映射 `osv_command_rejected`
  - 映射 `osv_scan_failed`
  - 映射 `osv_report_parse_failed`
  - 映射 `osv_export_failed`
  - 映射 `invalid_report_format`
  - 映射 `osv_ignore_update_failed`
  - 验收：前端可稳定显示业务错误

---

## M7 Tauri 桌面入口

- [x] OSV-0701 扩展桌面 settings
  - 新增 `OsvProjectSettings`
  - 新增 `OsvScannerSettings`
  - 在 `DesktopSettings` 中加入 `osv_scanner`
  - 保存 `command_history`
  - 验收：旧 settings 可通过 `serde(default)` 兼容

- [x] OSV-0702 注册 Tauri commands
  - `get_osv_settings`
  - `save_osv_settings`
  - `check_osv_installed`
  - `preview_osv_scan_command`
  - `scan_osv_project`
  - `preview_osv_report_export_command`
  - `export_osv_report`
  - `ignore_osv_vulnerability`
  - 验收：commands 仅做桥接，不实现业务逻辑

- [x] OSV-0703 限制命令历史条数
  - 默认保留最近 50 条
  - 保存 settings 时自动裁剪
  - 验收：settings 不因历史无限增长而膨胀

---

## M8 前端 API 适配层

- [x] OSV-0801 新增 `frontend/src/api/osvScanner.ts`
  - 定义前端类型
  - 封装 Tauri `invoke`
  - 封装 Web `fetch`
  - 统一解析错误格式
  - 验收：页面和 Store 不直接调用 `fetch` 或 `invoke`

- [x] OSV-0802 实现 settings 读写
  - Tauri 走 commands
  - Web 先走 localStorage
  - 裁剪 command_history
  - 验收：桌面和 Web 行为一致

- [x] OSV-0803 实现扫描与导出 API
  - `checkOsvInstalled`
  - `previewOsvScanCommand`
  - `scanOsvProject`
  - `previewOsvReportExportCommand`
  - `exportOsvReport`
  - `ignoreOsvVulnerability`
  - 验收：所有后端能力通过单一适配层暴露

---

## M9 Pinia Store

- [x] OSV-0901 新增 `frontend/src/stores/osvScanner.ts`
  - 管理项目列表
  - 管理安装状态
  - 管理当前命令预览
  - 管理扫描状态
  - 管理当前详情
  - 管理最近扫描摘要
  - 管理命令历史
  - 验收：全局配置有唯一权威源

- [x] OSV-0902 实现项目管理动作
  - 添加项目
  - 移除项目
  - 更新项目 last_scanned
  - 更新项目 health_score
  - 保存 settings
  - 验收：刷新后项目配置可恢复

- [x] OSV-0903 实现命令历史动作
  - 追加扫描记录
  - 追加导出记录
  - 保留最近 50 条
  - 支持复制命令所需数据
  - 验收：扫描和导出均可追溯

---

## M10 Vue 页面与导航

- [x] OSV-1001 新增 `frontend/src/pages/OsvScanner.vue`
  - 使用 `ToolShell`
  - 不做 landing page
  - 首屏就是工具工作台
  - 验收：页面符合现有视觉和布局体系

- [x] OSV-1002 实现顶部状态区
  - 安装状态
  - 全局健康分
  - 漏洞汇总
  - 最近扫描时间
  - 验收：用户打开页面能看到整体安全态势

- [x] OSV-1003 实现项目列表
  - 桌面端使用目录选择器
  - Web 端支持手动输入路径
  - 支持扫描、详情、移除
  - 验收：桌面和 Web 都能添加项目

- [x] OSV-1004 实现命令预览确认区
  - 展示 binary、working_dir、argv、display_command
  - 展示 locked args
  - 支持结构化参数调整
  - 支持确认执行
  - 支持复制命令
  - 验收：扫描不会绕过预览直接执行

- [x] OSV-1005 实现扫描结果详情
  - 按严重级别排序
  - 展示漏洞 ID、包名、版本、路径、修复建议
  - 支持忽略操作
  - 验收：用户能判断优先修复项

- [x] OSV-1006 实现报告导出区
  - JSON/HTML 格式选择
  - 输出路径确认
  - 导出命令预览
  - 导出完成提示
  - 验收：两种格式均可导出并追溯命令

- [x] OSV-1007 实现命令历史区
  - 展示最近 50 条
  - 展示类型、状态、时间、退出码、摘要
  - 支持查看完整 argv
  - 支持复制 display_command
  - 验收：用户可追溯当时执行的命令

- [x] OSV-1008 接入导航
  - 修改 `frontend/src/stores/tools.ts`
  - 修改 `frontend/src/router/index.ts`
  - 修改 `frontend/src/pages/Toolbox.vue`
  - 验收：侧边栏和工具箱首页都有入口

---

## M11 样式与可访问性

- [x] OSV-1101 补充全局语义样式
  - `health-good`
  - `health-warning`
  - `health-danger`
  - 漏洞严重级别样式
  - 命令状态样式
  - 验收：不在页面内大面积硬编码颜色

- [x] OSV-1102 优化响应式布局
  - 桌面宽屏项目列表和详情并排
  - 小屏抽屉或纵向堆叠
  - 命令字符串长文本可换行或横向滚动
  - 验收：移动和桌面无文本溢出或遮挡
  - 验证：浏览器检查 1280px 与 390px 视口均无横向溢出，控制台无错误

- [x] OSV-1103 图标与可访问标签
  - 扫描、导出、忽略、移除、复制使用 lucide 图标
  - 图标按钮提供 `aria-label` 或 tooltip
  - 验收：按钮含义清晰

---

## M12 测试与验证

- [x] OSV-1201 Rust core 单元测试
  - JSON 解析
  - 健康评分
  - 路径校验
  - 命令预览
  - 命令校验
  - 执行记录
  - JSON/HTML 导出命令
  - 忽略规则 TOML
  - 验收：`cargo test -p rust_tool_core` 通过

- [x] OSV-1202 Web route 测试
  - install-status
  - scan preview
  - scan
  - export preview
  - export
  - ignore
  - 错误格式
  - 验收：`cargo test -p rust_tool_server` 通过

- [x] OSV-1203 前端 API/Store 测试
  - Tauri 分支 mock
  - Web fetch 分支 mock
  - settings 读写
  - command history 裁剪
  - 错误解析
  - 验收：`pnpm run test:run` 通过

- [x] OSV-1204 构建验证
  - Rust 改动至少运行 `cargo test`
  - 前端改动至少运行 `pnpm run build`
  - 验收：无编译错误，无明显 warning

- [x] OSV-1205 手动集成验证
  - 使用隔离数据目录启动 Web
  - 检查安装状态
  - 添加测试项目
  - 预览命令
  - 执行扫描
  - 导出 JSON
  - 导出 HTML
  - 写入忽略规则
  - 重新扫描验证忽略生效
  - 验收：MVP 闭环可用
  - 验证：`RUSTTOOL_DATA_DIR=/private/tmp/rusttool-verify ./rt dev`，使用 `/private/tmp/rusttool-osv-vuln-fixture` 触发 7 个漏洞；导出 JSON 与 HTML；写入 `osv-scanner.toml` 后复扫降至 6 个漏洞，被忽略 ID 不再出现

---

## M13 第二阶段预留任务

- [ ] OSV-1301 一键修复命令预览
  - 验证 `osv-scanner fix` 当前版本参数
  - 生成修复命令预览
  - UI 明确提示会修改锁文件

- [ ] OSV-1302 一键修复执行
  - 执行前二次确认
  - 返回变更摘要
  - 写入命令历史

- [ ] OSV-1303 后台定时扫描
  - daily
  - weekly
  - none
  - 失败重试与状态记录

- [ ] OSV-1304 macOS 系统通知
  - 使用 Tauri 官方通知插件
  - 权限处理
  - 新增 Critical/High 漏洞提醒

- [ ] OSV-1305 扫描历史趋势
  - 保存摘要
  - 比较健康分变化
  - 展示最近扫描趋势

---

## MVP 完成标准

- [x] 桌面与 Web 共用同一套 Vue 页面。
- [x] 前端只通过 `frontend/src/api/osvScanner.ts` 调用后端能力。
- [x] Tauri 与 Axum 都只作为薄入口调用 `rust_tool_core`。
- [x] 用户执行扫描前可以查看命令。
- [x] 用户可以在受控范围内调整扫描参数。
- [x] 扫描执行后可以追溯当时实际执行的命令。
- [x] 应用内扫描结果来自 JSON 结构化解析。
- [x] 用户可以选择导出 JSON 或 HTML 报告。
- [x] 用户可以写入漏洞忽略规则。
- [x] `cargo test` 与 `pnpm run build` 通过。
