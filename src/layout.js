/**
 * layout.js - 显示器布局管理模块
 *
 * 封装多显示器布局相关的纯函数和 UI 渲染：
 * - 布局方向常量
 * - 包围盒计算（纯函数，可测试）
 * - 显示器方块渲染
 * - 布局预览更新
 */

// ─── 布局常量 ─────────────────────────────────────────────────────────────────

/** 布局方向到触发边缘的映射 */
const LAYOUT_TRIGGER_EDGE = {
    'LocalLeftOfRemote':  '右边缘',
    'LocalRightOfRemote': '左边缘',
    'LocalAboveRemote':   '下边缘',
    'LocalBelowRemote':   '上边缘',
};

/** 布局方向到预览 CSS 类的映射 */
const LAYOUT_PREVIEW_CLASS = {
    'LocalLeftOfRemote':  'layout-left',
    'LocalRightOfRemote': 'layout-right',
    'LocalAboveRemote':   'layout-above',
    'LocalBelowRemote':   'layout-below',
};

/** 布局方向到分隔线箭头的映射 */
const LAYOUT_DIVIDER_ARROW = {
    'LocalLeftOfRemote':  '→',
    'LocalRightOfRemote': '←',
    'LocalAboveRemote':   '↓',
    'LocalBelowRemote':   '↑',
};

// ─── 纯函数：包围盒计算 ───────────────────────────────────────────────────────

/**
 * 计算一组显示器的包围盒（纯函数）
 * @param {Array} monitors - 显示器数组
 * @returns {{ minX: number, minY: number, totalW: number, totalH: number }}
 */
function calcBoundingBox(monitors) {
    if (!monitors || monitors.length === 0) {
        return { minX: 0, minY: 0, totalW: 1920, totalH: 1080 };
    }
    let minX = Infinity, minY = Infinity, maxX = -Infinity, maxY = -Infinity;
    for (const m of monitors) {
        const ox = m.offset_x || 0;
        const oy = m.offset_y || 0;
        minX = Math.min(minX, ox);
        minY = Math.min(minY, oy);
        maxX = Math.max(maxX, ox + m.width);
        maxY = Math.max(maxY, oy + m.height);
    }
    return { minX, minY, totalW: maxX - minX, totalH: maxY - minY };
}

// ─── 纯函数：拖拽布局计算 ────────────────────────────────────────────────────

/**
 * 根据拖拽后本机和远端的中心坐标，计算布局方向（纯函数）
 * 优先判断距离更大的轴方向
 * @param {number} localCx - 本机中心 X
 * @param {number} localCy - 本机中心 Y
 * @param {number} remoteCx - 远端中心 X
 * @param {number} remoteCy - 远端中心 Y
 * @returns {string} 布局方向枚举值
 */
function calcLayoutFromDrop(localCx, localCy, remoteCx, remoteCy) {
    const dx = localCx - remoteCx;
    const dy = localCy - remoteCy;
    // 优先判断绝对值更大的轴
    if (Math.abs(dx) >= Math.abs(dy)) {
        return dx < 0 ? 'LocalLeftOfRemote' : 'LocalRightOfRemote';
    } else {
        return dy < 0 ? 'LocalAboveRemote' : 'LocalBelowRemote';
    }
}

/**
 * 检测两个矩形是否重叠（纯函数）
 * @param {{ x: number, y: number, w: number, h: number }} r1
 * @param {{ x: number, y: number, w: number, h: number }} r2
 * @returns {boolean}
 */
function isOverlapping(r1, r2) {
    return r1.x < r2.x + r2.w &&
           r1.x + r1.w > r2.x &&
           r1.y < r2.y + r2.h &&
           r1.y + r1.h > r2.y;
}

// ─── UI：拖拽交互 ─────────────────────────────────────────────────────────────

/**
 * 初始化显示器布局拖拽交互
 * 使用 mousedown/mousemove/mouseup 实现真正的拖拽（参考 Synergy/Barrier 设计）
 * 拖拽本机显示器方块，放下后自动吸附到远端的上下左右边缘
 * @param {Function} onLayoutChange - 布局变化回调 (newLayout: string) => void
 */
function initLayoutDrag(onLayoutChange) {
    const localGroup = document.getElementById('local-monitor-group');
    const remoteGroup = document.getElementById('remote-monitor-group');
    const preview = document.getElementById('display-layout-preview');
    if (!localGroup || !remoteGroup || !preview) return;

    let isDragging = false;
    let dragOffsetX = 0;
    let dragOffsetY = 0;
    let originalPosition = null; // 记录拖拽前的位置，用于恢复

    // 拖拽开始
    localGroup.addEventListener('mousedown', (e) => {
        e.preventDefault();
        isDragging = true;

        const rect = localGroup.getBoundingClientRect();
        dragOffsetX = e.clientX - rect.left;
        dragOffsetY = e.clientY - rect.top;

        // 记录原始位置
        originalPosition = {
            left: localGroup.style.left,
            top: localGroup.style.top,
            position: localGroup.style.position,
        };

        // 切换到绝对定位以便自由移动
        const previewRect = preview.getBoundingClientRect();
        localGroup.style.position = 'absolute';
        localGroup.style.left = (rect.left - previewRect.left) + 'px';
        localGroup.style.top = (rect.top - previewRect.top) + 'px';
        localGroup.style.zIndex = '10';
        localGroup.style.opacity = '0.85';
        localGroup.style.cursor = 'grabbing';
        localGroup.style.transition = 'none';

        // 添加拖拽中的视觉反馈
        localGroup.classList.add('dragging');
        preview.classList.add('drag-active');
    });

    // 拖拽移动（绑定到 document 以支持鼠标移出预览区域）
    document.addEventListener('mousemove', (e) => {
        if (!isDragging) return;
        e.preventDefault();

        const previewRect = preview.getBoundingClientRect();
        let newLeft = e.clientX - previewRect.left - dragOffsetX;
        let newTop = e.clientY - previewRect.top - dragOffsetY;

        // 限制在预览区域内
        const localW = localGroup.offsetWidth;
        const localH = localGroup.offsetHeight;
        newLeft = Math.max(-localW / 2, Math.min(newLeft, previewRect.width - localW / 2));
        newTop = Math.max(-localH / 2, Math.min(newTop, previewRect.height - localH / 2));

        localGroup.style.left = newLeft + 'px';
        localGroup.style.top = newTop + 'px';
    });

    // 拖拽结束
    document.addEventListener('mouseup', (e) => {
        if (!isDragging) return;
        isDragging = false;

        // 移除拖拽视觉反馈
        localGroup.classList.remove('dragging');
        preview.classList.remove('drag-active');
        localGroup.style.zIndex = '';
        localGroup.style.opacity = '1';
        localGroup.style.cursor = 'grab';

        const previewRect = preview.getBoundingClientRect();
        const dropX = e.clientX - previewRect.left;
        const dropY = e.clientY - previewRect.top;

        // 计算远端中心
        const remoteRect = remoteGroup.getBoundingClientRect();
        const remoteCx = remoteRect.left + remoteRect.width / 2 - previewRect.left;
        const remoteCy = remoteRect.top + remoteRect.height / 2 - previewRect.top;

        // 计算本机中心
        const localCx = dropX - dragOffsetX + localGroup.offsetWidth / 2;
        const localCy = dropY - dragOffsetY + localGroup.offsetHeight / 2;

        // 检测是否完全重叠（无效位置）
        const localW = localGroup.offsetWidth;
        const localH = localGroup.offsetHeight;
        const remoteW = remoteGroup.offsetWidth;
        const remoteH = remoteGroup.offsetHeight;

        const localRect = { x: localCx - localW / 2, y: localCy - localH / 2, w: localW, h: localH };
        const remoteRectBox = { x: remoteCx - remoteW / 2, y: remoteCy - remoteH / 2, w: remoteW, h: remoteH };

        if (isOverlapping(localRect, remoteRectBox)) {
            // 无效位置：显示红色边框动画，恢复原位
            localGroup.classList.add('drag-invalid');
            setTimeout(() => localGroup.classList.remove('drag-invalid'), 800);
            // 恢复原始位置
            _resetLocalGroupPosition(localGroup);
            return;
        }

        // 计算新布局方向
        const newLayout = calcLayoutFromDrop(localCx, localCy, remoteCx, remoteCy);

        // 恢复 flex 布局（不再使用绝对定位）
        _resetLocalGroupPosition(localGroup);

        if (onLayoutChange) {
            onLayoutChange(newLayout);
        }
    });
}

/**
 * 重置本机显示器组的定位（从绝对定位恢复到 flex 布局）
 * @param {HTMLElement} localGroup
 */
function _resetLocalGroupPosition(localGroup) {
    localGroup.style.position = '';
    localGroup.style.left = '';
    localGroup.style.top = '';
    localGroup.style.zIndex = '';
    localGroup.style.transition = '';
}

/**
 * 渲染一组显示器方块到指定容器
 * @param {HTMLElement} groupEl - 容器元素
 * @param {Array} monitors - 显示器数组
 * @param {{ minX, minY, totalW, totalH }} bbox - 包围盒
 * @param {number} scale - 缩放比例
 * @param {boolean} isLocal - 是否为本机显示器
 */
function renderMonitorGroup(groupEl, monitors, bbox, scale, isLocal) {
    if (!groupEl) return;
    const MIN_SIZE = 40;
    const groupW = Math.max(Math.round(bbox.totalW * scale), MIN_SIZE);
    const groupH = Math.max(Math.round(bbox.totalH * scale), MIN_SIZE);

    groupEl.style.width = groupW + 'px';
    groupEl.style.height = groupH + 'px';

    // 使用 i18n 翻译标签（如果可用）
    const t = (window.i18n && window.i18n.t) ? window.i18n.t : (key) => key;
    const localLabel = t('labelLocalMonitor');
    const secondaryLabel = t('labelSecondaryMonitor');
    const remoteLabel = t('labelRemoteMonitor');
    const primaryTag = t('labelPrimaryTag');

    groupEl.innerHTML = monitors.map((m) => {
        const ox = (m.offset_x || 0) - bbox.minX;
        const oy = (m.offset_y || 0) - bbox.minY;
        const left = Math.round(ox * scale);
        const top = Math.round(oy * scale);
        const w = Math.max(Math.round(m.width * scale), MIN_SIZE);
        const h = Math.max(Math.round(m.height * scale), MIN_SIZE);

        const isPrimary = isLocal && m.is_primary;
        const cssClass = isLocal
            ? (isPrimary ? 'local-primary' : 'local-secondary')
            : 'remote-connected';

        const dpiPct = m.dpi_scale ? Math.round(m.dpi_scale * 100) : 100;
        const tooltip = `${m.width}×${m.height} | DPI: ${dpiPct}% | ${t('labelMonitorOffset')}: (${m.offset_x || 0}, ${m.offset_y || 0})`;

        const showLabel = w >= 50 && h >= 36;
        const showSize = w >= 55 && h >= 48;

        const label = isLocal ? (isPrimary ? localLabel : secondaryLabel) : remoteLabel;

        return `
            <div class="monitor-block ${cssClass}"
                 style="left:${left}px;top:${top}px;width:${w}px;height:${h}px;"
                 title="${tooltip}">
                ${showLabel ? `<span class="monitor-block-label">${label}</span>` : ''}
                ${isPrimary ? `<span class="monitor-primary-tag">${primaryTag}</span>` : ''}
                ${showSize ? `<span class="monitor-block-size">${m.width}×${m.height}</span>` : ''}
            </div>
        `;
    }).join('');
}

/**
 * 渲染多显示器布局预览
 * @param {Array} localMonitors - 本机显示器数组
 * @param {Array|null} remoteMonitors - 远端显示器数组
 * @param {string} layout - 布局方向枚举值
 */
function renderMonitorLayout(localMonitors, remoteMonitors, layout) {
    const localGroup = document.getElementById('local-monitor-group');
    const remoteGroup = document.getElementById('remote-monitor-group');
    const divider = document.getElementById('layout-divider');
    if (!localGroup || !remoteGroup) return;

    if (divider) {
        divider.textContent = LAYOUT_DIVIDER_ARROW[layout] || '→';
    }

    const localBBox = calcBoundingBox(localMonitors);
    const hasRemote = remoteMonitors && remoteMonitors.length > 0;
    const remoteBBoxRaw = hasRemote
        ? calcBoundingBox(remoteMonitors)
        : { minX: 0, minY: 0, totalW: 1920, totalH: 1080 };

    const preview = document.getElementById('display-layout-preview');
    const isVertical = layout === 'LocalAboveRemote' || layout === 'LocalBelowRemote';
    const containerW = preview ? preview.clientWidth - 16 : 400;
    const containerH = preview ? preview.clientHeight - 24 : 140;
    const dividerSize = 28;

    const totalPhysicalW = isVertical
        ? Math.max(localBBox.totalW, remoteBBoxRaw.totalW)
        : localBBox.totalW + remoteBBoxRaw.totalW;
    const totalPhysicalH = isVertical
        ? localBBox.totalH + remoteBBoxRaw.totalH
        : Math.max(localBBox.totalH, remoteBBoxRaw.totalH);

    const availW = isVertical ? containerW : containerW - dividerSize;
    const availH = isVertical ? containerH - dividerSize : containerH;

    const scaleX = availW / totalPhysicalW;
    const scaleY = availH / totalPhysicalH;
    const scale = Math.min(scaleX, scaleY, 1);

    renderMonitorGroup(localGroup, localMonitors, localBBox, scale, true);

    if (hasRemote) {
        renderMonitorGroup(remoteGroup, remoteMonitors, remoteBBoxRaw, scale, false);
    } else {
        // 使用 i18n 翻译
        const t = (window.i18n && window.i18n.t) ? window.i18n.t : (key) => key;
        const remoteLabel = t('labelRemoteMonitor');

        const placeholderW = Math.max(Math.round(remoteBBoxRaw.totalW * scale), 60);
        const placeholderH = Math.max(Math.round(remoteBBoxRaw.totalH * scale), 50);
        remoteGroup.style.width = placeholderW + 'px';
        remoteGroup.style.height = placeholderH + 'px';
        remoteGroup.innerHTML = `
            <div class="monitor-block remote-placeholder"
                 style="left:0;top:0;width:${placeholderW}px;height:${placeholderH}px;"
                 title="${remoteLabel}">
                <span class="monitor-block-label">${remoteLabel}</span>
            </div>
        `;
    }
}

/**
 * 更新布局预览：切换方向 CSS 类、更新提示文字、重新渲染方块
 * @param {string} layout - 布局方向
 * @param {HTMLElement} preview - 预览容器
 * @param {HTMLElement|null} hintEdge - 触发边缘提示元素
 */
function updateLayoutPreview(layout, preview, hintEdge) {
    if (!preview) return;
    const { t } = window.i18n;

    preview.classList.remove('layout-left', 'layout-right', 'layout-above', 'layout-below');
    const cls = LAYOUT_PREVIEW_CLASS[layout];
    if (cls) preview.classList.add(cls);

    if (hintEdge) {
        const edgeKeyMap = {
            'LocalLeftOfRemote':  'edgeRight',
            'LocalRightOfRemote': 'edgeLeft',
            'LocalAboveRemote':   'edgeBottom',
            'LocalBelowRemote':   'edgeTop',
        };
        const edgeKey = edgeKeyMap[layout] || 'edgeRight';
        hintEdge.textContent = t(edgeKey);
    }

    const localMonitors = window._localMonitors || null;
    const remoteMonitors = window._remoteMonitors || null;
    if (localMonitors) {
        renderMonitorLayout(localMonitors, remoteMonitors, layout);
    }
}

// ─── 导出 ────────────────────────────────────────────────────────────────────

const _layoutExports = {
    // 常量
    LAYOUT_TRIGGER_EDGE,
    LAYOUT_PREVIEW_CLASS,
    LAYOUT_DIVIDER_ARROW,
    // 纯函数
    calcBoundingBox,
    calcLayoutFromDrop,
    isOverlapping,
    // UI 渲染
    renderMonitorGroup,
    renderMonitorLayout,
    updateLayoutPreview,
    // 拖拽交互
    initLayoutDrag,
};

// 兼容浏览器全局和 Node.js 测试环境
if (typeof module !== 'undefined' && module.exports) {
    module.exports = _layoutExports;
} else {
    window.LayoutModule = _layoutExports;
}
