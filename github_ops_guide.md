# GitHub 网页端操作指南

本指南详细介绍了在使用本项目的 GitHub Actions 自动构建工作流时，需要在 GitHub 网页端进行的各项设置与操作步骤。

---

## 1. 开启 GitHub Actions 读写权限（首要步骤）

默认情况下，GitHub Actions 的临时凭证仅有只读权限。若不开启写权限，打包任务在尝试创建 Release 和上传 `.dmg`/`.msi` 文件时会报错失败。

### 📌 操作步骤：
1. 打开您的浏览器，登录 GitHub 并进入项目的 **Repository (仓库) 主页**。
2. 在仓库顶部菜单栏中，点击右侧的 **⚙️ Settings (设置)**。
3. 在左侧侧边栏中，找到 **Actions** 菜单并点击展开，然后选择 **General (常规)**。
4. 滚动页面至最底部的 **Workflow permissions (工作流权限)** 区域。
5. 将选项从默认的 *Read repository content and packages permissions* 改为 **Read and write permissions**。
6. 勾选下方 **Allow GitHub Actions to create and approve pull requests**（允许 GitHub Actions 创建和批准 PR，可选）。
7. 点击右侧的 **Save (保存)** 按钮。

> [!IMPORTANT]
> 如果此权限未正确开启，构建日志中会出现类似 `HttpError: Resource not accessible by integration` 的报错，导致无法生成发布包。

---

## 2. 如何在网页端手动触发打包 (Manual Trigger)

我们在工作流中配置了 `workflow_dispatch`，这允许您在不发布新 Tag 的情况下，随时手动启动打包任务，非常适合测试编译或给同事提供临时版本。

### 📌 操作步骤：
1. 进入 GitHub 仓库主页，点击顶部菜单栏的 **Actions** 标签页。
2. 在左侧的 **Workflows** 列表中，点击 **Release Tauri App**。
3. 此时页面右上方会出现一个浅灰色的 **Run workflow** 按钮。
4. 点击 **Run workflow**，在弹出的菜单中：
   * **Use workflow from**：选择您想要编译的分支（例如 `master` 或 `main`）。
5. 点击绿色的 **Run workflow** 按钮启动任务。
6. 稍等几秒，页面会自动刷新并显示一个正在运行的构建任务（带有黄色旋转图标）。点击它可以查看实时的编译日志。

> [!TIP]
> 整个编译过程（包括 macOS 和 Windows）通常需要 10 ~ 20 分钟（取决于依赖缓存情况）。您可以关闭网页，编译完成后 GitHub 会发送邮件通知。

---

## 3. 如何获取编译好的安装包 (Download Releases)

构建成功后，工作流会自动在仓库中创建一个 **Draft (草稿) Release**，其中包含了打包出来的所有文件。

### 📌 获取步骤：
1. 任务运行完毕（显示绿色勾号）后，返回仓库主页。
2. 在右侧边栏 of the **Releases** 区域，您会看到一个标有 `Draft`（草稿）或最新版本的入口，点击进入。
3. 在 Release 页面最下方的 **Assets** 列表中，即可找到自动生成的各平台安装包：
   * **macOS**: `.dmg` 或 `.app.tar.gz` 文件
   * **Windows**: `.msi` 或 `.exe` 文件
4. 直接点击文件名称即可下载，并可以将链接分享或发送给同事。
5. **发布 (可选)**：如果您想正式发布该版本给所有人看，可以点击右上角 **Edit**，然后点击 **Publish release** 将草稿转为正式发布版。
