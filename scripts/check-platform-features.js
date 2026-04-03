#!/usr/bin/env node
/**
 * check-platform-features.js - 跨平台功能一致性检查脚本（JS 版本）
 * 用法：node scripts/check-platform-features.js
 *
 * 检查项：
 * 1. 手动连接功能（connect_by_ip）
 * 2. 连接历史记录功能
 * 3. 网络诊断功能（延迟监测、断线重连）
 * 4. 监听地址配置功能（set_listen_address、get_listen_config）
 */

const fs = require('fs');
const path = require('path');

const REPO_ROOT = path.join(__dirname, '..');
const PLATFORM_RS = path.join(REPO_ROOT, 'src-tauri/src/platform.rs');
const MAIN_RS = path.join(REPO_ROOT, 'src-tauri/src/main.rs');
const PLATFORM_JS = path.join(REPO_ROOT, 'src/platform.js');
const MAIN_JS = path.join(REPO_ROOT, 'src/main.js');
const INDEX_HTML = path.join(REPO_ROOT, 'src/index.html');

let issues = 0;
const results = [];

function check(name, condition, detail) {
    if (condition) {
        results.push({ status: '✅', name, detail: '' });
    } else {
        results.push({ status: '❌', name, detail });
        issues++;
    }
}

function readFile(filePath) {
    try {
        return fs.readFileSync(filePath, 'utf-8');
    } catch (e) {
        return '';
    }
}

console.log('\n═══════════════════════════════════════════════════');
console.log('  KVMini 跨平台功能一致性检查（JS 版本）');
console.log('═══════════════════════════════════════════════════\n');

const platformRs = readFile(PLATFORM_RS);
const mainRs = readFile(MAIN_RS);
const platformJs = readFile(PLATFORM_JS);
const mainJs = readFile(MAIN_JS);
const indexHtml = readFile(INDEX_HTML);

// ─── 1. 手动连接功能检查 ───
console.log('[1/4] 检查手动连接功能...');

check(
    'Rust: connect_by_ip 命令已定义',
    platformRs.includes('pub async fn connect_by_ip'),
    'platform.rs 中缺少 connect_by_ip 命令定义'
);
check(
    'Rust: connect_by_ip 已注册到 main.rs',
    mainRs.includes('connect_by_ip'),
    'main.rs 中未注册 connect_by_ip 命令'
);
check(
    'Rust: IPv4 格式校验函数 is_valid_ipv4 已实现',
    platformRs.includes('fn is_valid_ipv4'),
    'platform.rs 中缺少 is_valid_ipv4 函数'
);
check(
    'Rust: 区分 refused/timeout/unreachable 三种错误类型',
    platformRs.includes('"refused"') && platformRs.includes('"timeout"') && platformRs.includes('"unreachable"'),
    'platform.rs 中缺少错误类型区分'
);
check(
    'Rust: 5 秒超时设置',
    platformRs.includes('from_secs(5)'),
    'platform.rs 中缺少 5 秒超时设置'
);
check(
    'JS: Platform.connectByIp 已封装',
    platformJs.includes("safeInvoke('connect_by_ip'"),
    'platform.js 中缺少 connectByIp 封装'
);
check(
    'UI: 手动连接 IP 输入框存在',
    indexHtml.includes('id="manual-ip-input"'),
    'index.html 中缺少 manual-ip-input 元素'
);
check(
    'UI: 手动连接端口输入框存在',
    indexHtml.includes('id="manual-port-input"'),
    'index.html 中缺少 manual-port-input 元素'
);
check(
    'UI: 手动连接按钮存在',
    indexHtml.includes('id="manual-connect-btn"'),
    'index.html 中缺少 manual-connect-btn 元素'
);
check(
    'JS: handleManualConnect 函数已实现',
    mainJs.includes('async function handleManualConnect'),
    'main.js 中缺少 handleManualConnect 函数'
);

// ─── 2. 连接历史记录功能检查 ───
console.log('\n[2/4] 检查连接历史记录功能...');

check(
    'JS: 手动 IP 历史记录写入（saveManualIpToHistory）已实现',
    mainJs.includes('saveManualIpToHistory'),
    'main.js 中缺少 saveManualIpToHistory 函数'
);
check(
    'JS: 历史记录区分手动/自动设备（entryType）',
    mainJs.includes("entryType: 'manual'") && mainJs.includes("entryType: 'auto'"),
    'main.js 中缺少 entryType 区分逻辑'
);
check(
    'JS: 手动 IP 使用 🔗 图标，自动发现使用 📡 图标',
    mainJs.includes('🔗') && mainJs.includes('📡'),
    'main.js 中缺少图标区分逻辑'
);
check(
    'JS: 历史记录最多保留 20 条',
    mainJs.includes('slice(0, 20)') || mainJs.includes('trimmed.slice(0, 20)'),
    'main.js 中缺少 20 条限制逻辑'
);
check(
    'JS: 手动 IP 一键重连（reconnectManualIp）已实现',
    mainJs.includes('async function reconnectManualIp'),
    'main.js 中缺少 reconnectManualIp 函数'
);

// ─── 3. 网络诊断功能检查 ───
console.log('\n[3/4] 检查网络诊断功能...');

check(
    'JS: 延迟监测（startLatencyCheck）已实现',
    mainJs.includes('function startLatencyCheck'),
    'main.js 中缺少 startLatencyCheck 函数'
);
check(
    'JS: 延迟 > 50ms 黄色警告色',
    mainJs.includes('> 50') && mainJs.includes('#d97706'),
    'main.js 中缺少延迟超阈值警告色逻辑'
);
check(
    'JS: 断线重连最多 3 次',
    mainJs.includes('MAX_RECONNECT_ATTEMPTS') && mainJs.includes('= 3'),
    'main.js 中缺少最大重连次数限制'
);
check(
    'JS: 重连间隔 2 秒',
    mainJs.includes('RECONNECT_INTERVAL_MS') && mainJs.includes('2000'),
    'main.js 中缺少重连间隔设置'
);
check(
    'JS: 重连失败后通知用户',
    mainJs.includes('handleConnectionLost'),
    'main.js 中缺少 handleConnectionLost 函数'
);

// ─── 4. 监听地址配置功能检查 ───
console.log('\n[4/4] 检查监听地址配置功能...');

check(
    'Rust: get_network_interfaces 命令已定义',
    platformRs.includes('pub fn get_network_interfaces'),
    'platform.rs 中缺少 get_network_interfaces 命令'
);
check(
    'Rust: Windows (Preferred) 后缀处理',
    platformRs.includes('trim_end_matches("(Preferred)")'),
    'platform.rs 中缺少 Windows (Preferred) 后缀处理'
);
check(
    'Rust: set_listen_address 命令已定义',
    platformRs.includes('pub fn set_listen_address'),
    'platform.rs 中缺少 set_listen_address 命令'
);
check(
    'Rust: get_listen_config 命令已定义',
    platformRs.includes('pub fn get_listen_config'),
    'platform.rs 中缺少 get_listen_config 命令'
);
check(
    'Rust: 端口范围校验（1024-65535）',
    platformRs.includes('port < 1024'),
    'platform.rs 中缺少端口范围校验'
);
check(
    'Rust: set_listen_address 已注册到 main.rs',
    mainRs.includes('set_listen_address'),
    'main.rs 中未注册 set_listen_address 命令'
);
check(
    'Rust: get_listen_config 已注册到 main.rs',
    mainRs.includes('get_listen_config'),
    'main.rs 中未注册 get_listen_config 命令'
);
check(
    'JS: Platform.getNetworkInterfaces 已封装',
    platformJs.includes("safeInvoke('get_network_interfaces'"),
    'platform.js 中缺少 getNetworkInterfaces 封装'
);
check(
    'JS: Platform.setListenAddress 已封装',
    platformJs.includes("safeInvoke('set_listen_address'"),
    'platform.js 中缺少 setListenAddress 封装'
);
check(
    'JS: Platform.getListenConfig 已封装',
    platformJs.includes("safeInvoke('get_listen_config'"),
    'platform.js 中缺少 getListenConfig 封装'
);
check(
    'UI: 监听网卡下拉列表存在',
    indexHtml.includes('id="listen-ip-select"'),
    'index.html 中缺少 listen-ip-select 元素'
);
check(
    'UI: 监听端口输入框存在',
    indexHtml.includes('id="listen-port-input"'),
    'index.html 中缺少 listen-port-input 元素'
);
check(
    'UI: 对端连接地址显示存在',
    indexHtml.includes('id="listen-address-display"'),
    'index.html 中缺少 listen-address-display 元素'
);
check(
    'JS: loadListenConfig 函数已实现',
    mainJs.includes('async function loadListenConfig'),
    'main.js 中缺少 loadListenConfig 函数'
);

// ─── 结果汇总 ───
console.log('\n═══════════════════════════════════════════════════');
console.log('  检查结果');
console.log('═══════════════════════════════════════════════════');

results.forEach(r => {
    if (r.status === '✅') {
        console.log(`  ${r.status} ${r.name}`);
    } else {
        console.log(`  ${r.status} ${r.name}`);
        console.log(`     → ${r.detail}`);
    }
});

console.log('\n═══════════════════════════════════════════════════');
if (issues === 0) {
    console.log(`  ✅ 所有 ${results.length} 项检查通过！跨平台功能完整。`);
} else {
    console.log(`  ❌ 发现 ${issues} 个问题，请补全后再提交代码。`);
}
console.log('═══════════════════════════════════════════════════\n');

process.exit(issues);
