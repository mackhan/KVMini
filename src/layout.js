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
 * 支持 HTML5 Drag & Drop API
 * @param {Function} onLayoutChange - 布局变化回调 (newLayout: string) => void
 */
function initLayoutDrag(onLayoutChange) {
    const localGroup = document.getElementById('local-monitor-group');
    const remoteGroup = document.getElementById('remote-monitor-group');
    const preview = document.getElementById('display-layout-preview');
    if (!localGroup || !remoteGroup || !preview) return;

    let dragStartX = 0;
    let dragStartY = 0;
    let lastValidLayout = null;

    // 拖拽开始
    localGroup.addEventListener('dragstart', (e) => {
        const rect = localGroup.getBoundingClientRect();
        dragStartX = rect.left + rect.width / 2;
        dragStartY = rect.top + rect.height / 2;
        localGroup.style.opacity = '0.6';
        if (e.dataTransfer) {
            e.dataTransfer.effectAllowed = 'move';
        }
    });

    // 拖拽结束
    localGroup.addEventListener('dragend', () => {
        localGroup.style.opacity = '1';
        localGroup.classList.remove('drag-invalid');
    });

    // 拖拽经过预览区域
    preview.addEventListener('dragover', (e) => {
        e.preventDefault();
        if (e.dataTransfer) e.dataTransfer.dropEffect = 'move';
    });

    // 放置到预览区域
    preview.addEventListener('drop', (e) => {
        e.preventDefault();
        const previewRect = preview.getBoundingClientRect();
        const dropX = e.clientX - previewRect.left;
        const dropY = e.clientY - previewRect.top;

        // 计算远端中心
        const remoteRect = remoteGroup.getBoundingClientRect();
        const remoteCx = remoteRect.left + remoteRect.width / 2 - previewRect.left;
        const remoteCy = remoteRect.top + remoteRect.height / 2 - previewRect.top;

        // 检测是否完全重叠（无效位置）
        const localW = parseInt(localGroup.style.width) || 80;
        const localH = parseInt(localGroup.style.height) || 60;
        const remoteW = parseInt(remoteGroup.style.width) || 80;
        const remoteH = parseInt(remoteGroup.style.height) || 60;

        const localRect = { x: dropX - localW / 2, y: dropY - localH / 2, w: localW, h: localH };
        const remoteRectBox = { x: remoteCx - remoteW / 2, y: remoteCy - remoteH / 2, w: remoteW, h: remoteH };

        if (isOverlapping(localRect, remoteRectBox)) {
            // 无效位置：显示红色边框，恢复上一个有效位置
            localGroup.classList.add('drag-invalid');
            setTimeout(() => localGroup.classList.remove('drag-invalid'), 800);
            return;
        }

        // 计算新布局方向
        const newLayout = calcLayoutFromDrop(dropX, dropY, remoteCx, remoteCy);
        lastValidLayout = newLayout;

        if (onLayoutChange) {
            onLayoutChange(newLayout);
        }
    });
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
        const tooltip = `${m.width}×${m.height} | DPI: ${dpiPct}% | 偏移: (${m.offset_x || 0}, ${m.offset_y || 0})`;

        const showLabel = w >= 50 && h >= 36;
        const showSize = w >= 55 && h >= 48;

        return `
            <div class="monitor-block ${cssClass}"
                 style="left:${left}px;top:${top}px;width:${w}px;height:${h}px;"
                 title="${tooltip}">
                ${showLabel ? `<span class="monitor-block-label">${isLocal ? (isPrimary ? '本机' : '副屏') : '远端'}</span>` : ''}
                ${isPrimary ? `<span class="monitor-primary-tag">主</span>` : ''}
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
        const placeholderW = Math.max(Math.round(remoteBBoxRaw.totalW * scale), 60);
        const placeholderH = Math.max(Math.round(remoteBBoxRaw.totalH * scale), 50);
        remoteGroup.style.width = placeholderW + 'px';
        remoteGroup.style.height = placeholderH + 'px';
        remoteGroup.innerHTML = `
            <div class="monitor-block remote-placeholder"
                 style="left:0;top:0;width:${placeholderW}px;height:${placeholderH}px;"
                 title="远端设备（未连接）">
                <span class="monitor-block-label">远端</span>
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

window.LayoutModule = {
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
