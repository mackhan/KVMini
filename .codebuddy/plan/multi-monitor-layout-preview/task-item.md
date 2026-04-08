# 实施计划：多显示器布局预览重新设计

> 本计划基于 `requirements.md` 需求文档，仅涉及前端 UI 层（HTML/CSS/JS）的修改，不涉及后端 Rust 代码。
> 涉及文件：`src/index.html`、`src/styles.css`、`src/main.js`

---

- [x] 1. 重构布局预览区域的 HTML 结构
   - 修改 `src/index.html` 中 `#display-layout-preview` 区域，移除现有的固定 `#local-display` 和 `#remote-display` 两个静态方块
   - 改为两个容器 div：`#local-monitor-group`（本机显示器组）和 `#remote-monitor-group`（远端显示器组），内容由 JS 动态填充
   - 在两个容器之间添加 `#layout-divider` 分隔线元素（用于显示方向箭头）
   - 预览容器改为 `position: relative`，内部方块使用绝对定位以支持真实物理排列
   - _需求：1.1, 1.2, 1.6, 2.2, 5.6_

- [x] 2. 重写布局预览区域的 CSS 样式
   - 修改 `src/styles.css`，将 `.display-layout-preview` 从 `display: flex` 改为 `position: relative` 容器，高度自适应（min-height: 120px, max-height: 200px）
   - 移除 `.display-block` 的固定 `width: 80px; height: 60px`，改为由 JS 动态计算并通过 inline style 设置
   - 新增 `.monitor-group` 样式（`position: relative`），用于包裹一组显示器方块
   - 新增 `.local-monitor`（深蓝色，主显示器）和 `.local-monitor-secondary`（浅蓝色，副显示器）样式
   - 新增 `.remote-monitor`（深灰色，已连接）和 `.remote-monitor-placeholder`（浅灰色虚线边框，未连接占位）样式
   - 新增 `.layout-divider` 样式（虚线 + 方向箭头文字）
   - 保留 `.layout-left`、`.layout-right`、`.layout-above`、`.layout-below` 方向类，但改为控制两个 group 的排列方式（flex-direction）
   - _需求：4.1, 4.2, 4.5, 5.1, 5.2, 5.6_

- [x] 3. 实现多显示器等比例缩放渲染核心函数
   - 在 `src/main.js` 中新增 `renderMonitorLayout(monitors, remoteMonitors, layout)` 函数
   - 该函数接收本机显示器数组、远端显示器数组（可为 null）、布局方向
   - 计算所有显示器的总包围盒（bounding box），根据预览容器尺寸计算统一缩放比例
   - 为每块显示器生成一个 div，通过 `position: absolute` + `left/top/width/height` 定位，反映真实的 offset_x/offset_y 物理排列
   - 每个方块内显示分辨率文字（如 `3840×2160`），主显示器标注"主"标签
   - 缩放后方块最小尺寸限制为 40px（宽或高）
   - _需求：1.1, 1.3, 1.4, 1.6, 4.1, 4.3, 4.4_

- [x] 4. 改造 `refreshMonitors()` 函数，将显示器数据存储到全局变量并触发布局渲染
   - 修改 `src/main.js` 中的 `refreshMonitors()` 函数
   - 将 `get_monitors` 返回的显示器数组存储到 `window._localMonitors` 全局变量
   - 移除现有的 `localSizeEl.textContent = ...` 单显示器尺寸更新逻辑
   - 在获取数据后调用 `renderMonitorLayout()` 重新渲染布局预览
   - _需求：1.5, 6.1, 6.3_

- [x] 5. 改造 `initDisplayLayout()` 函数，适配新的多显示器渲染流程
   - 修改 `src/main.js` 中的 `initDisplayLayout()` 函数
   - 移除对 `#local-display`、`#local-display-size` 等旧静态元素的引用
   - 加载布局配置后，调用 `renderMonitorLayout()` 进行初始渲染
   - 下拉框 change 事件中调用 `renderMonitorLayout()` 而非 `updateLayoutPreview()`
   - _需求：6.1, 6.2, 6.4_

- [x] 6. 改造 `updateLayoutPreview()` 函数，支持新的多显示器布局
   - 修改 `src/main.js` 中的 `updateLayoutPreview()` 函数
   - 保留布局方向 CSS 类切换和触发边缘提示文字更新逻辑
   - 新增：更新 `#layout-divider` 分隔线的方向箭头（← → ↑ ↓）
   - 新增：调用 `renderMonitorLayout()` 重新渲染方块位置
   - _需求：3.1, 3.2, 3.3, 3.4, 3.5, 5.6_

- [x] 7. 适配拖拽交互逻辑
   - 修改 `src/main.js` 中 `initDisplayLayout()` 的拖拽相关代码
   - 将拖拽源从单个 `#local-display` 改为整个 `#local-monitor-group` 容器
   - drop 事件中的布局方向判断逻辑保持不变（基于落点相对于预览中心的位置）
   - drop 后调用 `renderMonitorLayout()` 重新渲染
   - _需求：3.6_

- [x] 8. 实现远端显示器占位与未来数据接入
   - 在 `renderMonitorLayout()` 中处理远端显示器数据：
     - 若 `remoteMonitors` 为 null 或空数组，渲染一个灰色占位方块（标注"远端"，虚线边框）
     - 若 `remoteMonitors` 有数据，按同样的等比例缩放逻辑渲染远端显示器组
   - 新增 `window._remoteMonitors` 全局变量，预留远端显示器数据接入点
   - _需求：2.1, 2.2, 2.3, 2.4, 6.5_

- [x] 9. 添加 tooltip 悬停交互
   - 在 `renderMonitorLayout()` 生成的每个显示器方块上添加 `title` 属性或自定义 tooltip
   - tooltip 内容包含：分辨率（width×height）、DPI 缩放（如 150%）、偏移量（offset_x, offset_y）
   - 使用原生 `title` 属性实现，保持轻量
   - _需求：5.3_

- [x] 10. 适配深色主题样式
   - 在 `src/styles.css` 的 `[data-theme="dark"]` 区块中添加新增样式的深色主题覆盖
   - `.display-layout-preview` 深色背景（如 `#1f2937`）
   - `.local-monitor` / `.local-monitor-secondary` 深色主题下的蓝色调整
   - `.remote-monitor-placeholder` 深色主题下的虚线边框颜色调整
   - `.layout-divider` 深色主题下的虚线和箭头颜色调整
   - _需求：5.4, 5.5_

- [x] 11. 改造 `saveLayoutConfig()` 函数，保存多显示器信息
   - 修改 `src/main.js` 中的 `saveLayoutConfig()` 函数
   - 将 `local_screen_width/height` 从 `window.screen.width/height` 改为使用主显示器的真实分辨率（从 `window._localMonitors` 获取）
   - 保持配置格式向后兼容（`layout`、`local_screen_width`、`local_screen_height` 字段不变）
   - _需求：6.2, 6.4_
