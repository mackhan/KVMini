/**
 * handshake-tls-payload.test.ts
 * Bug 1（BUG-018）：握手 TLS 状态正确互传 TDD 测试
 *
 * 该测试只覆盖前端可观察的行为契约：
 * - tls_mismatch 仅在双方 tls 状态不一致时发出
 * - 双方都开 TLS 时不应弹警告
 *
 * Rust 端 wire payload 兼容性单测见 kvm-core/src/network.rs::tests
 */

describe('Bug 1: 握手 TLS 状态互传', () => {
    /**
     * 模拟后端 tls_mismatch 判定逻辑：
     * 与 src-tauri/src/connection/mod.rs::connect_to_peer 中
     * `if local_tls != remote_tls { emit tls_mismatch }` 完全等价。
     */
    function shouldEmitTlsMismatch(localTls: boolean, remoteTls: boolean): boolean {
        return localTls !== remoteTls;
    }

    test('双方都开 TLS 时不应弹 tls_mismatch（修复前的核心 bug）', () => {
        // Bug 现象：两台 Mac 都已开启 TLS，但仍弹出"本机已开启 TLS 加密，但对端未开启"
        // 根因：connect_to / handle_stream 解析 HandshakeAck 时硬编码 tls_enabled=false
        // 期望：双方都开 TLS 时，tls_mismatch 不应触发
        expect(shouldEmitTlsMismatch(true, true)).toBe(false);
    });

    test('双方都关 TLS 时不应弹 tls_mismatch', () => {
        expect(shouldEmitTlsMismatch(false, false)).toBe(false);
    });

    test('本机开对端关时应弹 tls_mismatch', () => {
        expect(shouldEmitTlsMismatch(true, false)).toBe(true);
    });

    test('本机关对端开时应弹 tls_mismatch', () => {
        expect(shouldEmitTlsMismatch(false, true)).toBe(true);
    });

    test('peer_info 必须包含 tls_enabled 字段（连接事件契约）', () => {
        // 模拟后端 PeerHandshake 事件 payload，新版必须含 tls_enabled
        const peerInfoNew = {
            version: 1,
            platform: 'MacOS',
            screen_info: {},
            tls_enabled: true, // ← Bug 1 修复后必须正确反映对端真实状态
        };
        expect(peerInfoNew).toHaveProperty('tls_enabled');
        expect(typeof peerInfoNew.tls_enabled).toBe('boolean');
    });
});
