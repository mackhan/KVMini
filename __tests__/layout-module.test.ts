/**
 * layout-module.test.ts
 * 测试 src/layout.js 中的显示器布局纯函数
 */

// 加载模块
const LayoutModule = require('../src/layout.js');

beforeAll(() => {
    // 同时挂载到 window 上，兼容旧测试
    (window as any).LayoutModule = LayoutModule;
});

describe('layout.js - calcBoundingBox', () => {
    test('单显示器应返回正确包围盒', () => {
        const monitors = [{ width: 1920, height: 1080, offset_x: 0, offset_y: 0 }];
        const bbox = (window as any).LayoutModule.calcBoundingBox(monitors);
        expect(bbox.totalW).toBe(1920);
        expect(bbox.totalH).toBe(1080);
        expect(bbox.minX).toBe(0);
        expect(bbox.minY).toBe(0);
    });

    test('双显示器横向排列应返回合并包围盒', () => {
        const monitors = [
            { width: 1920, height: 1080, offset_x: 0, offset_y: 0 },
            { width: 2560, height: 1440, offset_x: 1920, offset_y: 0 },
        ];
        const bbox = (window as any).LayoutModule.calcBoundingBox(monitors);
        expect(bbox.totalW).toBe(4480);
        expect(bbox.totalH).toBe(1440);
        expect(bbox.minX).toBe(0);
    });

    test('双显示器纵向排列应返回合并包围盒', () => {
        const monitors = [
            { width: 1920, height: 1080, offset_x: 0, offset_y: 0 },
            { width: 1920, height: 1080, offset_x: 0, offset_y: 1080 },
        ];
        const bbox = (window as any).LayoutModule.calcBoundingBox(monitors);
        expect(bbox.totalW).toBe(1920);
        expect(bbox.totalH).toBe(2160);
    });

    test('空数组应返回默认值 1920x1080', () => {
        const bbox = (window as any).LayoutModule.calcBoundingBox([]);
        expect(bbox.totalW).toBe(1920);
        expect(bbox.totalH).toBe(1080);
    });

    test('null 应返回默认值', () => {
        const bbox = (window as any).LayoutModule.calcBoundingBox(null);
        expect(bbox.totalW).toBe(1920);
        expect(bbox.totalH).toBe(1080);
    });

    test('有偏移的显示器应正确计算 minX/minY', () => {
        const monitors = [
            { width: 1920, height: 1080, offset_x: 100, offset_y: 50 },
        ];
        const bbox = (window as any).LayoutModule.calcBoundingBox(monitors);
        expect(bbox.minX).toBe(100);
        expect(bbox.minY).toBe(50);
        expect(bbox.totalW).toBe(1920);
        expect(bbox.totalH).toBe(1080);
    });
});

describe('layout.js - LAYOUT_TRIGGER_EDGE', () => {
    test('应包含所有四个方向', () => {
        const edges = (window as any).LayoutModule.LAYOUT_TRIGGER_EDGE;
        expect(edges['LocalLeftOfRemote']).toBeDefined();
        expect(edges['LocalRightOfRemote']).toBeDefined();
        expect(edges['LocalAboveRemote']).toBeDefined();
        expect(edges['LocalBelowRemote']).toBeDefined();
    });
});

describe('layout.js - LAYOUT_DIVIDER_ARROW', () => {
    test('LocalLeftOfRemote 应使用右箭头', () => {
        const arrows = (window as any).LayoutModule.LAYOUT_DIVIDER_ARROW;
        expect(arrows['LocalLeftOfRemote']).toBe('→');
    });

    test('LocalRightOfRemote 应使用左箭头', () => {
        const arrows = (window as any).LayoutModule.LAYOUT_DIVIDER_ARROW;
        expect(arrows['LocalRightOfRemote']).toBe('←');
    });

    test('LocalAboveRemote 应使用下箭头', () => {
        const arrows = (window as any).LayoutModule.LAYOUT_DIVIDER_ARROW;
        expect(arrows['LocalAboveRemote']).toBe('↓');
    });

    test('LocalBelowRemote 应使用上箭头', () => {
        const arrows = (window as any).LayoutModule.LAYOUT_DIVIDER_ARROW;
        expect(arrows['LocalBelowRemote']).toBe('↑');
    });
});
