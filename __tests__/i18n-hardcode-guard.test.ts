/**
 * i18n-hardcode-guard.test.ts
 * BUG-021 / plan v0.1.38 组 A3：i18n 守卫测试
 *
 * 目的：防止开发者在 src/main.js / src/index.html 里新增硬编码中文，
 *      强制走 src/i18n.js 的 t() / data-i18n 机制。
 *
 * 豁免：注释、console/log 日志、诊断日志（diagLog / write_startup_log）、
 *      极个别字面量（由 whitelist 明确列出并注释解释）。
 */

const fs = require('fs');
const path = require('path');

const mainJs = fs.readFileSync(path.join(__dirname, '../src/main.js'), 'utf-8');
const indexHtml = fs.readFileSync(path.join(__dirname, '../src/index.html'), 'utf-8');
const i18nJs = fs.readFileSync(path.join(__dirname, '../src/i18n.js'), 'utf-8');

const CJK_RE = /[\u4e00-\u9fa5]/;

/**
 * 按行扫描，返回含中文的"非豁免"行
 *
 * 多行 data-i18n 元素处理：HTML 里经常出现：
 *   <p data-i18n="xxx">
 *       多行
 *       兜底内容
 *   </p>
 * 此时「多行兜底内容」所在的行不含 data-i18n，但父元素含有——此处用简单栈
 * 判断：若最近一个尚未关闭的标签带有 data-i18n，则当前行豁免。
 */
function findHardcodedLines(src: string, options: {
    stripComments?: boolean;
    stripConsoleLog?: boolean;
    stripDataI18n?: boolean;
    trackOpenDataI18n?: boolean;
    whitelist?: RegExp[];
}): Array<{ line: number; content: string }> {
    const {
        stripComments = true,
        stripConsoleLog = true,
        stripDataI18n = true,
        trackOpenDataI18n = false,
        whitelist = [],
    } = options;
    const lines = src.split('\n');
    const result: Array<{ line: number; content: string }> = [];

    // 跟踪当前是否处于某个带 data-i18n 的元素内部
    let dataI18nOpenDepth = 0;

    for (let i = 0; i < lines.length; i++) {
        const raw = lines[i];

        // 先更新 data-i18n 开闭栈（不管是否含中文都要跟踪）
        if (trackOpenDataI18n) {
            // 粗略检测：带 data-i18n 的起始标签 且 本行未自闭合/未闭合
            const hasDataI18nOpen = /<[a-zA-Z][^>]*\bdata-i18n(?:-[a-z]+)?\s*=\s*"[^"]+"/i.test(raw);
            // 粗略检测：本行含有闭合标签 </xxx>
            const closeTagMatch = raw.match(/<\/[a-zA-Z][^>]*>/);
            if (hasDataI18nOpen && !/\/>\s*$/.test(raw) && !closeTagMatch) {
                dataI18nOpenDepth++;
            } else if (closeTagMatch && dataI18nOpenDepth > 0 && !hasDataI18nOpen) {
                dataI18nOpenDepth--;
            } else if (hasDataI18nOpen && closeTagMatch) {
                // 自同行开闭，不修改栈
            }
        }

        if (!CJK_RE.test(raw)) continue;

        const trimmed = raw.trim();

        // 豁免：单行注释
        if (stripComments) {
            if (trimmed.startsWith('//') || trimmed.startsWith('*') || trimmed.startsWith('/*') || trimmed.startsWith('<!--')) {
                continue;
            }
            // 豁免：行内注释里的中文（例如 `resolve(null); // 超时降级`）
            // 判断：注释符号之前没有中文
            const commentIdx = raw.indexOf('//');
            if (commentIdx >= 0) {
                const beforeComment = raw.slice(0, commentIdx);
                if (!CJK_RE.test(beforeComment)) continue;
            }
        }

        // 豁免：log/console 输出
        if (stripConsoleLog) {
            if (/\b(console\.(log|warn|error|debug|info)|log\.(info|warn|error|debug)|diagLog|write_startup_log)\s*\(/.test(raw)) {
                continue;
            }
        }

        // 豁免：data-i18n / data-i18n-placeholder / data-i18n-title / data-title-i18n 行
        // 也允许标签跨 2 行的情况（本行 + 下一行）
        if (stripDataI18n) {
            const nextLine = lines[i + 1] || '';
            const combined = raw + '\n' + nextLine;
            if (/data-i18n(?:-[a-z]+)?\s*=/.test(combined) || /data-title-i18n\s*=/.test(combined)) {
                continue;
            }
        }

        // 豁免：当前处于 data-i18n 开放元素内部（多行兜底内容）
        if (trackOpenDataI18n && dataI18nOpenDepth > 0) {
            continue;
        }

        // 豁免：whitelist 正则
        if (whitelist.some(re => re.test(raw))) {
            continue;
        }

        result.push({ line: i + 1, content: trimmed });
    }

    return result;
}

describe('i18n 守卫：禁止新增硬编码中文', () => {
    test('src/main.js 不允许面向用户的硬编码中文字符串', () => {
        const bad = findHardcodedLines(mainJs, {
            whitelist: [
                // 辅助函数中的 i18n 结构化 replace（为 overlayDesc 加粗，绑定 i18n 原文）
                /replace\('辅助功能（Accessibility）权限'/,
                // 诊断日志：logs.push(...) / invoke('write_startup_log', ...) — 非 UI 面向
                /logs\.push\(/,
                /invoke\('write_startup_log'/,
            ],
        });
        if (bad.length > 0) {
            const msg = bad.map(b => `  L${b.line}: ${b.content}`).join('\n');
            throw new Error(`src/main.js 出现 ${bad.length} 处硬编码中文：\n${msg}\n\n请改为 window.i18n.t('key') 并在 src/i18n.js 同时补齐 zh/en 两个字典。`);
        }
        expect(bad).toHaveLength(0);
    });

    test('src/index.html 不允许非 data-i18n 标注的硬编码中文', () => {
        const bad = findHardcodedLines(indexHtml, {
            trackOpenDataI18n: true,
            whitelist: [
                // HTML 注释
                /^<!--/,
                // 前一行已带 data-i18n-placeholder，本行因跨行只看到 placeholder="xxx"
                /^\s*placeholder="[^"]+"\s*$/,
                // 前一行已带 data-i18n，本行仅为跨行收尾的中文文本（由 JS t() 覆盖）
                /^\s*对端首次连接时需核对此指纹，确认一致后方可信任连接/,
                // overlayDesc 跨 3 行兜底（<p data-i18n="overlayDesc">...</p>），
                // 由 JS updateOverlayI18n() 用 t('overlayDesc') 动态填充 innerHTML
                /KVMini 需要<strong>辅助功能（Accessibility）权限<\/strong>才能捕获和注入键盘鼠标事件/,
                /^\s*请在系统设置中授权后，应用将自动继续。\s*$/,
            ],
        });
        if (bad.length > 0) {
            const msg = bad.map(b => `  L${b.line}: ${b.content}`).join('\n');
            throw new Error(`src/index.html 出现 ${bad.length} 处未标注 data-i18n 的硬编码中文：\n${msg}\n\n请为元素添加 data-i18n="key"（文本）或 data-i18n-placeholder="key"（placeholder），并在 src/i18n.js 同时补齐 zh/en 两个字典。`);
        }
        expect(bad).toHaveLength(0);
    });

    test("src/i18n.js 所有 key 必须在 zh 和 en 两个字典里都有定义", () => {
        // 用 MESSAGES 对象解析
        const { MESSAGES } = require('../src/i18n.js');
        const zhKeys = new Set(Object.keys(MESSAGES.zh));
        const enKeys = new Set(Object.keys(MESSAGES.en));

        const zhOnly = [...zhKeys].filter(k => !enKeys.has(k));
        const enOnly = [...enKeys].filter(k => !zhKeys.has(k));

        if (zhOnly.length > 0 || enOnly.length > 0) {
            throw new Error(
                `i18n key 两端不一致：\n` +
                (zhOnly.length ? `仅 zh 存在（缺 en）：${zhOnly.join(', ')}\n` : '') +
                (enOnly.length ? `仅 en 存在（缺 zh）：${enOnly.join(', ')}` : '')
            );
        }

        expect(zhOnly).toHaveLength(0);
        expect(enOnly).toHaveLength(0);
    });

    test('plan v0.1.38 列出的关键 key 必须存在', () => {
        const { MESSAGES } = require('../src/i18n.js');
        const requiredKeys = [
            // plan A2 列出的
            'detailLocal', 'detailPeer', 'detailEncryption', 'detailLatency', 'detailCertFingerprint',
            'btnReconnect', 'tabGeneral', 'tabDisplay', 'tabNetwork', 'tabSecurity', 'tabAbout',
            'licenseActivated', 'statusDisconnected', 'labelReconnectProgress', 'reconnectingText',
            'labelControlState', 'controlStateIdle', 'labelConnectedDevice', 'labelConnectionType',
            'labelLocalIp', 'labelLatency', 'detailTitle', 'peerAddressPrefix',
            // plan A2 新增硬编码迁移 key
            'errTauriNotReady', 'errTauriInitFailed', 'errDisconnectPrefix', 'errReconnectFailPrefix',
            'errSettingsSavePrefix', 'errHotkeyRegisterPrefix', 'errDisableTlsRetryPrefix',
            'msgConnectionLostDefault', 'msgReconnectSuccessDefault', 'msgTlsMismatchDefault',
            'msgReconnectFailRetrying', 'msgConnectedToIp',
            'labelLocalIpShort', 'latencyTipHigh', 'latencyTipCritical',
            'tlsBadgeOn', 'tlsBadgeOff', 'notifyDisconnectedManualRetry',
            'scanningSubnetTip', 'monitorRemoteDisconnected', 'monitorTooltipTemplate',
            'certRegenSuccess', 'certRegenFailed', 'shareText',
            'nicDescriptionHint', 'nicDisconnectedInlineText',
        ];

        const missing = requiredKeys.filter(k => !(k in MESSAGES.zh) || !(k in MESSAGES.en));
        if (missing.length > 0) {
            throw new Error(`i18n.js 缺少以下关键 key（zh 或 en）：${missing.join(', ')}`);
        }
        expect(missing).toHaveLength(0);
    });
});
