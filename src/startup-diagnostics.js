/**
 * startup-diagnostics.js - 启动时自动诊断日志模块
 *
 * 在 build 版本启动时将关键状态写入本地日志文件，
 * 开发者无需用户操作即可通过查看日志验证修复是否生效。
 *
 * 日志路径：~/Library/Logs/KVMini/startup.log
 *
 * 需求：1.1、1.2、1.3、1.4、1.5
 */

/** 日志条目缓冲区 */
const _logBuffer = [];

/** 启动时间戳 */
const _startTime = Date.now();

/**
 * 记录一条诊断日志（缓冲到内存，flush 时写入文件）
 * @param {string} level - 日志级别：INFO / WARN / ERROR / FATAL
 * @param {string} message - 日志内容
 */
function log(level, message) {
    const elapsed = Date.now() - _startTime;
    const entry = `[${new Date().toISOString()}] [+${elapsed}ms] [${level}] ${message}`;
    _logBuffer.push(entry);
    // 同时输出到 console，方便开发调试
    if (level === 'FATAL' || level === 'ERROR') {
        console.error('[Diagnostics]', entry);
    } else if (level === 'WARN') {
        console.warn('[Diagnostics]', entry);
    } else {
        console.log('[Diagnostics]', entry);
    }
}

/**
 * 将缓冲区日志写入文件
 * 通过 invoke 调用 Rust 后端的 write_startup_log 命令；
 * 非 Tauri 环境降级到 console.error（需求 1.5）
 * @returns {Promise<void>}
 */
async function flush() {
    if (_logBuffer.length === 0) return;

    const content = _logBuffer.join('\n') + '\n';

    // 尝试通过 invoke 写入日志（需要 Tauri 环境）
    const invoke = window.__kvminiInvoke
        || window.__TAURI__?.core?.invoke
        || window.__TAURI__?.invoke
        || window.__TAURI_INTERNALS__?.invoke;
    if (invoke) {
        try {
            await invoke('write_startup_log', { content });
            return;
        } catch (e) {
            // invoke 失败时降级到 console.error（需求 1.5）
            console.error('[Diagnostics] invoke write_startup_log 失败，降级输出:', e, '\n' + content);
            return;
        }
    }

    // 非 Tauri 环境：降级到 console.error（需求 1.5）
    console.error('[Diagnostics] 非 Tauri 环境，日志降级输出:\n' + content);
}

/**
 * 记录启动诊断快照（在 waitForTauri 完成后调用）
 * @param {object} snapshot - 诊断快照
 * @param {boolean} snapshot.tauriReady - window.__TAURI__ 是否就绪
 * @param {boolean} snapshot.invokeReady - invoke 是否就绪
 * @param {number} snapshot.waitElapsed - waitForTauri 等待耗时（ms）
 * @param {string|null} snapshot.permissionStatus - 权限检测结果
 * @param {string|null} snapshot.overlayState - 遮罩最终状态（shown/hidden）
 */
async function logStartupSnapshot(snapshot) {
    log('INFO', `=== KVMini 启动诊断 ===`);
    log('INFO', `window.__TAURI__ 就绪: ${snapshot.tauriReady}`);
    log('INFO', `invoke 就绪: ${snapshot.invokeReady}`);
    log('INFO', `waitForTauri 耗时: ${snapshot.waitElapsed}ms`);

    if (!snapshot.invokeReady) {
        // 需求 1.2：invoke 为 null 时写入 FATAL 日志
        log('FATAL', 'invoke is null after waitForTauri');
    }

    if (snapshot.permissionStatus !== null) {
        log('INFO', `权限检测结果: ${snapshot.permissionStatus}`);
    } else {
        log('WARN', '权限检测未执行（invoke 未就绪）');
    }

    if (snapshot.overlayState !== null) {
        log('INFO', `权限遮罩状态: ${snapshot.overlayState}`);
    }

    log('INFO', `=== 诊断结束 ===`);
    await flush();
}

// ─── 导出 ────────────────────────────────────────────────────────────────────

// CommonJS 导出（供 Jest 测试使用）
if (typeof module !== 'undefined' && module.exports) {
    module.exports = { log, flush, logStartupSnapshot };
}

// 浏览器/Tauri 全局导出
window.StartupDiagnostics = { log, flush, logStartupSnapshot };
