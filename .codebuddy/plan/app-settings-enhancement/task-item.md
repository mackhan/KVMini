# 实施计划

- [ ] 1. 升级版本号（三处同步）
   - 将 `package.json`、`src-tauri/Cargo.toml`、`src-tauri/tauri.conf.json` 中的版本号统一从 `0.1.3` 改为 `0.1.4`
   - _需求：2.3_

- [ ] 2. 修复应用图标
   - 使用 Canvas/脚本生成有效的 `32x32.png`、`128x128.png`、`icon.icns`、`icon.ico` 图标文件，替换 `src-tauri/icons/` 下的空文件
   - 确保图标文件大小 > 1KB，打包后 Dock 和系统托盘可正常显示
   - _需求：1.1、1.2、1.3_

- [ ] 3. 实现 i18n 多语言模块
   - 在 `src/` 下新建 `i18n.js`，定义中英文两套文本字典（涵盖所有按钮、标签、提示、设置项、权限引导遮罩文本）
   - 导出 `t(key)` 翻译函数和 `setLocale(lang)` / `getLocale()` 方法
   - _需求：3.1、3.4、3.5、3.6_

- [ ] 4. 实现主题切换模块
   - 在 `src/` 下新建 `theme.js`，实现 `setTheme(theme)` / `getTheme()` 方法
   - `setTheme` 根据参数在 `<html>` 上设置 `data-theme="dark|light"`，"跟随系统"时监听 `prefers-color-scheme` 媒体查询
   - 在 `src/styles.css` 中补充 `[data-theme="dark"]` 和 `[data-theme="light"]` 的 CSS 变量覆盖规则
   - _需求：4.1、4.2、4.6、4.7、4.8_

- [ ] 5. 在设置面板 UI 中增加语言、主题、版本号展示
   - 在 `index.html` 设置面板中新增"语言"选择器（中文/English）和"主题"选择器（深色/浅色/跟随系统）
   - 在设置面板底部和主界面页脚新增版本号显示区域（`v0.1.4`）
   - _需求：2.1、2.2、3.1、4.1_

- [ ] 6. 在 main.js 中集成语言、主题、版本号逻辑
   - 页面加载时：从 `localStorage` 读取已保存的语言和主题设置并立即应用（避免主题闪烁）
   - 绑定语言/主题选择器的 `change` 事件，调用 `setLocale` / `setTheme` 并刷新所有文本
   - 版本号优先从 `window.__TAURI__.app.getVersion()` 读取，降级使用 HTML 静态值
   - 保存设置时将语言和主题写入 `localStorage`（key: `kvm_settings`）
   - _需求：2.4、3.2、3.3、4.3、4.4、4.5_

- [ ] 7. 权限引导遮罩文本国际化
   - 将辅助功能权限引导遮罩中的所有硬编码文本替换为 `t(key)` 调用
   - 确保语言切换时遮罩文本同步更新
   - _需求：3.7_

- [ ] 8. 编写测试用例
   - 为 `i18n.js` 编写单元测试：语言切换、`t()` 返回值、默认语言、持久化
   - 为 `theme.js` 编写单元测试：主题切换、`data-theme` 属性设置、跟随系统逻辑、持久化
   - 为版本号显示逻辑编写测试：Tauri 环境可用时读取动态版本、降级读取静态版本
   - 运行 `npm test` 确保全部通过
   - _需求：5.1、5.2、5.3、5.4_
