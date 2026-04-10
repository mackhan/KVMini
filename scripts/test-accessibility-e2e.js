#!/usr/bin/env node
/**
 * test-accessibility-e2e.js - 可在 build 版本中运行的专项测试脚本
 *
 * 通过读取 ~/Library/Logs/KVMini/startup.log 验证关键状态，
 * 无需用户手动操作即可验证权限引导和按钮点击功能是否正常。
 *
 * 使用方式：
 *   1. 先启动 build 版本（让应用写入诊断日志）
 *   2. 运行 npm run test:e2e:accessibility
 *
 * 需求：4.1、4.2、4.3
 */

const fs = require('fs');
const path = require('path');
const os = require('os');

// 诊断日志路径
const LOG_PATH = path.join(os.homedir(), 'Library', 'Logs', 'KVMini', 'startup.log');

// 测试结果收集
const results = [];
let passed = 0;
let failed = 0;

/**
 * 断言函数
 */
function assert(condition, testName, failReason) {
    if (condition) {
        console.log(`  ✅ [PASS] ${testName}`);
        results.push({ name: testName, passed: true });
        passed++;
    } else {
        console.error(`  ❌ [FAIL] ${testName}`);
        if (failReason) console.error(`         原因: ${failReason}`);
        results.push({ name: testName, passed: false, reason: failReason });
        failed++;
    }
}

/**
 * 读取诊断日志
 */
function readDiagnosticsLog() {
    if (!fs.existsSync(LOG_PATH)) {
        return null;
    }
    try {
        return fs.readFileSync(LOG_PATH, 'utf-8');
    } catch (e) {
        return null;
    }
}

/**
 * 主测试函数
 */
async function runTests() {
    console.log('\n🔍 KVMini 权限引导 + 按钮点击专项测试');
    console.log('='.repeat(50));
    console.log(`📄 诊断日志路径: ${LOG_PATH}\n`);

    // ── 检查日志文件是否存在 ──
    console.log('📋 测试组 1：诊断日志文件');
    const logContent = readDiagnosticsLog();

    assert(
        logContent !== null,
        '诊断日志文件存在',
        `日志文件不存在: ${LOG_PATH}\n请先启动 build 版本，等待应用完全加载后再运行此测试`
    );

    if (!logContent) {
        printSummary();
        return;
    }

    // ── 验证日志内容 ──
    console.log('\n📋 测试组 2：invoke 就绪状态');

    const hasFatalInvoke = logContent.includes('[FATAL] invoke is null after waitForTauri');
    const hasInvokeReady = logContent.includes('invoke 就绪: true') || logContent.includes('waitForTauri 成功');

    assert(
        !hasFatalInvoke,
        'invoke 在 waitForTauri 后就绪（无 FATAL 日志）',
        hasFatalInvoke
            ? 'invoke 为 null！Tauri 环境未正确初始化。请检查 withGlobalTauri 配置'
            : null
    );

    assert(
        hasInvokeReady,
        'waitForTauri 成功完成',
        !hasInvokeReady
            ? '日志中未找到 waitForTauri 成功记录，可能应用未正常启动'
            : null
    );

    // ── 验证权限检测 ──
    console.log('\n📋 测试组 3：权限检测');

    const hasPermissionCheck = logContent.includes('权限检测结果:');
    const hasPermissionGranted = logContent.includes('权限检测结果: Granted');
    const hasPermissionDenied = logContent.includes('权限检测结果: Denied');
    const hasOverlayState = logContent.includes('权限遮罩状态:');

    assert(
        hasPermissionCheck,
        'check_accessibility_permission 被调用',
        !hasPermissionCheck
            ? '日志中未找到权限检测记录，可能 invoke 未就绪导致权限检测被跳过'
            : null
    );

    if (hasPermissionCheck) {
        assert(
            hasPermissionGranted || hasPermissionDenied,
            '权限检测返回有效状态（Granted 或 Denied）',
            '权限检测返回了未知状态'
        );

        assert(
            hasOverlayState,
            '权限遮罩状态已记录',
            '日志中未找到权限遮罩状态记录'
        );

        if (hasPermissionGranted) {
            const hasOverlayHidden = logContent.includes('权限遮罩状态: hidden');
            assert(
                hasOverlayHidden,
                '权限已授权时遮罩应隐藏',
                '权限已授权但遮罩未隐藏！'
            );
        }

        if (hasPermissionDenied) {
            const hasOverlayShown = logContent.includes('权限遮罩状态: shown');
            assert(
                hasOverlayShown,
                '权限未授权时遮罩应显示',
                '权限未授权但遮罩未显示！'
            );
        }
    }

    // ── 验证按钮绑定 ──
    console.log('\n📋 测试组 4：按钮事件监听器');

    const hasButtonBindings = logContent.includes('按钮事件监听器绑定状态:');
    assert(
        hasButtonBindings,
        '按钮事件监听器绑定状态已记录',
        '日志中未找到按钮绑定状态记录'
    );

    if (hasButtonBindings) {
        // 检查关键按钮是否绑定成功
        const bindingLine = logContent.split('\n').find(l => l.includes('按钮事件监听器绑定状态:'));
        if (bindingLine) {
            const allBound = !bindingLine.includes(':false');
            assert(
                allBound,
                '所有关键按钮均已绑定事件监听器',
                allBound ? null : `部分按钮未绑定: ${bindingLine}`
            );
        }
    }

    printSummary();
}

/**
 * 打印测试摘要
 */
function printSummary() {
    console.log('\n' + '='.repeat(50));
    console.log(`📊 测试结果: ${passed} 通过, ${failed} 失败`);

    if (failed === 0) {
        console.log('\n✅ [PASS] 权限引导和按钮点击功能正常');
        process.exit(0);
    } else {
        console.error(`\n❌ [FAIL] ${failed} 个测试失败`);
        console.error(`📄 请查看诊断日志获取详细信息: ${LOG_PATH}`);

        // 打印失败的测试
        const failedTests = results.filter(r => !r.passed);
        failedTests.forEach(t => {
            console.error(`\n  ❌ ${t.name}`);
            if (t.reason) console.error(`     ${t.reason}`);
        });

        process.exit(1);
    }
}

// 运行测试
runTests().catch(e => {
    console.error('测试运行失败:', e);
    process.exit(1);
});
