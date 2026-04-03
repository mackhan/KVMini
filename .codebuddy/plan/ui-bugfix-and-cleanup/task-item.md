# 实施计划：UI修复、跨平台功能一致性与进程清理

- [ ] 1. 修复 Tauri invoke 解构错误
   - 将 `main.js` 顶部的 `const { invoke } = window.__TAURI__.tauri` 改为使用 `@tauri-apps/api/tauri` 模块导入
   - 确保所有 invoke 调用在 `DOMContentLoaded` 事件后执行
   - 编写测试用例验证页面加载时不出现 TypeError
   - _需求：1.1、1.2、1.3、1.4、1.5_

- [ ] 2. 审计两端功能差异，建立功能对照表
   - 扫描 `src-tauri/src/main.rs` 中所有 `#[tauri::command]` 注册的命令
   - 扫描前端 `src/main.js` 和 `src/index.html` 中所有已实现的功能模块
   - 整理出 Windows 版有而 macOS 版缺失（或反之）的功能清单，记录到 `docs/platform-feature-matrix.md`
   - _需求：2.1、2.2、2.3、2.4、2.5、2.6、2.7、2.8_

- [ ] 3. 补全统一的前端界面（主界面 + 设置面板）
   - 基于功能对照表，在 `src/index.html` 中补全缺失的 UI 组件：设备列表、连接状态、延迟显示、扫描动画、连接历史
   - 将 `alert` 替换为完整的设置面板，包含快捷键配置、连接偏好、启动选项
   - 确保界面在 macOS 和 Windows 上视觉一致（布局、颜色、字体）
   - 编写 UI 渲染测试用例，验证各组件正确显示
   - _需求：2.1、2.2、2.6、2.8_

- [ ] 4. 补全 Rust 后端跨平台命令（系统托盘、通知、快捷键）
   - 在 `src-tauri/src/main.rs` 中使用 `#[cfg(target_os)]` 条件编译，为 macOS 和 Windows 分别实现系统托盘、系统通知、全局快捷键命令
   - 确保两个平台暴露相同的 Tauri 命令接口（命令名称和参数一致）
   - 编写 Rust 单元测试，验证核心逻辑在两个平台上行为一致
   - _需求：2.3、2.4、2.7、3.1、3.2、3.3、3.6_

- [ ] 5. 实现平台抽象层，统一前端调用接口
   - 创建 `src/platform.js`，封装所有平台差异调用，前端其他模块只调用此抽象层
   - 对不支持的平台功能实现优雅降级（如 Windows 不支持某 macOS 特性时静默忽略）
   - 编写测试用例验证抽象层在两个平台上均可正常调用
   - _需求：3.3、3.4、3.5_

- [ ] 6. 补全连接历史记录功能
   - 在前端实现连接历史列表的展示与快速重连功能
   - 在 Rust 后端实现历史记录的持久化存储（读写本地文件）
   - 编写测试用例验证历史记录的增删查功能
   - _需求：2.5_

- [ ] 7. 清理 kvnshare 遗留进程
   - 在 `src-tauri/src/main.rs` 的应用启动逻辑中，添加检测并终止 `kvnshare` 进程的代码（使用条件编译区分平台）
   - 创建 `scripts/cleanup-legacy.sh`（macOS）和 `scripts/cleanup-legacy.ps1`（Windows）清理脚本，移除开机启动项
   - 编写测试用例验证启动时不存在 kvnshare 遗留进程
   - _需求：5.1、5.2、5.3、5.4、5.5_

- [ ] 8. 创建跨平台功能一致性检查脚本
   - 创建 `scripts/check-platform-parity.sh`，自动扫描两端 Tauri 命令和前端功能模块的差异
   - 配置 git post-merge hook（`.git/hooks/post-merge`），在 `git pull` 后自动运行检查脚本
   - 检查脚本发现缺失时输出警告和参考文件路径；一致时输出"跨平台功能检查通过"
   - _需求：4.1、4.2、4.3、4.4、4.5、4.6_

- [ ] 9. 全量测试与构建验证
   - 运行所有单元测试和集成测试，确保全部通过
   - 在 macOS 上执行 `npm run build` 构建 Release 版，验证无编译错误
   - 手动验证：应用启动无 TypeError、界面功能完整、托盘正常、无 kvnshare 遗留进程
   - _需求：1.5、2.1～2.8、3.6、5.5_
