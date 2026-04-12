/**
 * layout-drag.test.ts
 * 显示器布局拖拽功能 TDD 测试用例
 * 需求：4.1 ~ 4.6
 */

// 模拟 Tauri invoke
const mockInvoke = jest.fn().mockResolvedValue(null);
(global as any).__TAURI__ = { core: { invoke: mockInvoke } };

// 加载模块
const LayoutModule = require('../src/layout.js');

beforeAll(() => {
    // 同时挂载到 window 上，兼容旧测试
    (window as any).LayoutModule = LayoutModule;
});

describe('显示器布局拖拽功能 (LayoutDragModule)', () => {

    // ─── 4.1 可视化预览 ──────────────────────────────────────────────────────

    test('4.1 calcBoundingBox 应正确计算单显示器包围盒', () => {
        const monitors = [{ width: 1920, height: 1080, offset_x: 0, offset_y: 0 }];
        const bbox = (window as any).LayoutModule.calcBoundingBox(monitors);
        expect(bbox.totalW).toBe(1920);
        expect(bbox.totalH).toBe(1080);
    });

    // ─── 4.3 释放拖拽后自动计算触发边界 ─────────────────────────────────────

    test('4.3 calcLayoutFromDrop - 本机在左侧时应返回 LocalLeftOfRemote', () => {
        const LayoutModule = (window as any).LayoutModule;
        // 本机中心 x=100, 远端中心 x=300 → 本机在左
        const result = LayoutModule.calcLayoutFromDrop(100, 200, 300, 200);
        expect(result).toBe('LocalLeftOfRemote');
    });

    test('4.3 calcLayoutFromDrop - 本机在右侧时应返回 LocalRightOfRemote', () => {
        const LayoutModule = (window as any).LayoutModule;
        // 本机中心 x=400, 远端中心 x=100 → 本机在右
        const result = LayoutModule.calcLayoutFromDrop(400, 200, 100, 200);
        expect(result).toBe('LocalRightOfRemote');
    });

    test('4.3 calcLayoutFromDrop - 本机在上方时应返回 LocalAboveRemote', () => {
        const LayoutModule = (window as any).LayoutModule;
        // 本机中心 y=100, 远端中心 y=300 → 本机在上
        const result = LayoutModule.calcLayoutFromDrop(200, 100, 200, 300);
        expect(result).toBe('LocalAboveRemote');
    });

    test('4.3 calcLayoutFromDrop - 本机在下方时应返回 LocalBelowRemote', () => {
        const LayoutModule = (window as any).LayoutModule;
        // 本机中心 y=400, 远端中心 y=100 → 本机在下
        const result = LayoutModule.calcLayoutFromDrop(200, 400, 200, 100);
        expect(result).toBe('LocalBelowRemote');
    });

    // ─── 4.5 无效位置检测（完全重叠） ────────────────────────────────────────

    test('4.5 isOverlapping - 完全重叠时应返回 true', () => {
        const LayoutModule = (window as any).LayoutModule;
        // 两个矩形完全重叠
        const r1 = { x: 0, y: 0, w: 100, h: 100 };
        const r2 = { x: 0, y: 0, w: 100, h: 100 };
        expect(LayoutModule.isOverlapping(r1, r2)).toBe(true);
    });

    test('4.5 isOverlapping - 部分重叠时应返回 true', () => {
        const LayoutModule = (window as any).LayoutModule;
        const r1 = { x: 0, y: 0, w: 100, h: 100 };
        const r2 = { x: 50, y: 50, w: 100, h: 100 };
        expect(LayoutModule.isOverlapping(r1, r2)).toBe(true);
    });

    test('4.5 isOverlapping - 不重叠时应返回 false', () => {
        const LayoutModule = (window as any).LayoutModule;
        const r1 = { x: 0, y: 0, w: 100, h: 100 };
        const r2 = { x: 200, y: 0, w: 100, h: 100 };
        expect(LayoutModule.isOverlapping(r1, r2)).toBe(false);
    });

    test('4.5 isOverlapping - 相邻但不重叠时应返回 false', () => {
        const LayoutModule = (window as any).LayoutModule;
        const r1 = { x: 0, y: 0, w: 100, h: 100 };
        const r2 = { x: 100, y: 0, w: 100, h: 100 };
        expect(LayoutModule.isOverlapping(r1, r2)).toBe(false);
    });

    // ─── 4.6 显示器数量变化时重新渲染 ────────────────────────────────────────

    test('4.6 calcBoundingBox 多显示器应正确计算包围盒', () => {
        const LayoutModule = (window as any).LayoutModule;
        const monitors = [
            { width: 1920, height: 1080, offset_x: 0, offset_y: 0 },
            { width: 2560, height: 1440, offset_x: 1920, offset_y: 0 },
        ];
        const bbox = LayoutModule.calcBoundingBox(monitors);
        expect(bbox.totalW).toBe(4480);
        expect(bbox.totalH).toBe(1440);
    });

    // ─── 拖拽状态管理 ────────────────────────────────────────────────────────

    test('calcLayoutFromDrop - 水平距离大于垂直距离时应优先判断水平方向', () => {
        const LayoutModule = (window as any).LayoutModule;
        // 水平差 200, 垂直差 50 → 应判断水平方向
        const result = LayoutModule.calcLayoutFromDrop(100, 200, 300, 250);
        expect(['LocalLeftOfRemote', 'LocalRightOfRemote']).toContain(result);
    });

    test('calcLayoutFromDrop - 垂直距离大于水平距离时应优先判断垂直方向', () => {
        const LayoutModule = (window as any).LayoutModule;
        // 水平差 50, 垂直差 200 → 应判断垂直方向
        const result = LayoutModule.calcLayoutFromDrop(100, 100, 150, 300);
        expect(['LocalAboveRemote', 'LocalBelowRemote']).toContain(result);
    });

    // ─── 旧版降级拖拽逻辑方向语义验证 ────────────────────────────────────────
    // main.js 旧版降级逻辑使用 dx = relX - centerX（鼠标相对预览区中心偏移）
    // 语义：dx < 0 → 鼠标在中心左侧 → 本机拖到左边 → LocalLeftOfRemote
    //       dx > 0 → 鼠标在中心右侧 → 本机拖到右边 → LocalRightOfRemote
    //       dy < 0 → 鼠标在中心上方 → 本机拖到上方 → LocalAboveRemote
    //       dy > 0 → 鼠标在中心下方 → 本机拖到下方 → LocalBelowRemote

    test('旧版降级逻辑：鼠标在中心左侧时应返回 LocalLeftOfRemote（dx<0）', () => {
        const LayoutModule = (window as any).LayoutModule;
        // 模拟：本机中心在 x=100，远端中心在 x=300（本机在左）
        const result = LayoutModule.calcLayoutFromDrop(100, 200, 300, 200);
        expect(result).toBe('LocalLeftOfRemote');
    });

    test('旧版降级逻辑：鼠标在中心右侧时应返回 LocalRightOfRemote（dx>0）', () => {
        const LayoutModule = (window as any).LayoutModule;
        // 模拟：本机中心在 x=400，远端中心在 x=100（本机在右）
        const result = LayoutModule.calcLayoutFromDrop(400, 200, 100, 200);
        expect(result).toBe('LocalRightOfRemote');
    });

    test('旧版降级逻辑：鼠标在中心上方时应返回 LocalAboveRemote（dy<0）', () => {
        const LayoutModule = (window as any).LayoutModule;
        // 模拟：本机中心在 y=100，远端中心在 y=300（本机在上）
        const result = LayoutModule.calcLayoutFromDrop(200, 100, 200, 300);
        expect(result).toBe('LocalAboveRemote');
    });

    test('旧版降级逻辑：鼠标在中心下方时应返回 LocalBelowRemote（dy>0）', () => {
        const LayoutModule = (window as any).LayoutModule;
        // 模拟：本机中心在 y=400，远端中心在 y=100（本机在下）
        const result = LayoutModule.calcLayoutFromDrop(200, 400, 200, 100);
        expect(result).toBe('LocalBelowRemote');
    });

});
