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

我们在工作流中配置了 `workflow_dispatch`，这允许您在不发布正式版本 Tag 的情况下，随时手动启动打包任务，非常适合测试编译或给同事提供临时版本。

手动触发时，工作流会重建 `master-latest` 预发布 Release，把 `master-latest` 标签移动到当前分支提交，并将安装包上传到这个预发布版本。这样 Release 页面中的源码包和安装包会指向同一个构建提交，不会混入正式版本 `v0.1.0`、`v0.1.1` 等 Release。

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

## 3. 如何获取手动编译好的安装包 (Download master-latest)

构建成功后，工作流会自动重建一个名为 **RustTool master-latest** 的 **Pre-release (预发布)**，其中包含从当前分支打包出来的文件。

### 📌 获取步骤：
1. 任务运行完毕（显示绿色勾号）后，返回仓库主页。
2. 在右侧边栏的 **Releases** 区域，点击进入最新 Release 列表。
3. 找到 `master-latest` / **RustTool master-latest** 预发布版本。
4. 在 Release 页面最下方的 **Assets** 列表中，即可找到自动生成的各平台安装包：
   * **macOS**: `.dmg` 或 `.app.tar.gz` 文件
   * **Windows**: `.msi` 或 `.exe` 文件
5. 直接点击文件名称即可下载，并可以将链接分享或发送给同事。

> [!NOTE]
> `master-latest` 是移动标签，会随着新的手动打包而更新，只适合测试和预览，不适合作为正式版本归档。

---

## 4. 如何发布正式版本 (Versioned Release)

正式版本应通过 `v*` 标签触发，例如 `v0.1.1`。这种方式会创建对应版本的 Release，源码包和安装包都绑定到同一个标签提交，适合长期归档。

### 📌 获取步骤：
1. 推送正式版本标签，例如 `v0.1.1`。
2. 等待 **Release Tauri App** 工作流完成。
3. 在 Release 页面最下方的 **Assets** 列表中，即可找到自动生成的各平台安装包：
   * **macOS**: `.dmg` 或 `.app.tar.gz` 文件
   * **Windows**: `.msi` 或 `.exe` 文件
4. **发布 (可选)**：如果工作流创建的是草稿版本，可以点击右上角 **Edit**，然后点击 **Publish release** 将草稿转为正式发布版。
