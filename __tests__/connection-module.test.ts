/**
 * connection-module.test.ts
 * 测试 src/connection.js 中提取的纯函数
 */

// 加载被测模块
beforeAll(() => {
    // 模拟 window.i18n
    (window as any).i18n = {
        t: (key: string) => key,
    };
    // 模拟 window.Platform
    (window as any).Platform = {
        connectByIp: jest.fn(),
    };
    // 加载模块
    require('../src/connection.js');
});

describe('connection.js - isValidIpv4', () => {
    test('应接受合法 IPv4 地址', () => {
        expect((window as any).ConnectionModule.isValidIpv4('192.168.1.1')).toBe(true);
        expect((window as any).ConnectionModule.isValidIpv4('10.0.0.1')).toBe(true);
        expect((window as any).ConnectionModule.isValidIpv4('0.0.0.0')).toBe(true);
        expect((window as any).ConnectionModule.isValidIpv4('255.255.255.255')).toBe(true);
    });

    test('应拒绝超出范围的 IP', () => {
        expect((window as any).ConnectionModule.isValidIpv4('999.0.0.1')).toBe(false);
        expect((window as any).ConnectionModule.isValidIpv4('256.1.1.1')).toBe(false);
        expect((window as any).ConnectionModule.isValidIpv4('1.2.3.256')).toBe(false);
    });

    test('应拒绝格式错误的 IP', () => {
        expect((window as any).ConnectionModule.isValidIpv4('abc')).toBe(false);
        expect((window as any).ConnectionModule.isValidIpv4('')).toBe(false);
        expect((window as any).ConnectionModule.isValidIpv4('1.2.3')).toBe(false);
        expect((window as any).ConnectionModule.isValidIpv4('1.2.3.4.5')).toBe(false);
        expect((window as any).ConnectionModule.isValidIpv4('1.2.3.')).toBe(false);
    });

    test('应拒绝含非数字字符的 IP', () => {
        expect((window as any).ConnectionModule.isValidIpv4('1.2.3.a')).toBe(false);
        expect((window as any).ConnectionModule.isValidIpv4('1.2.3.-1')).toBe(false);
    });
});

describe('connection.js - isValidPort', () => {
    test('应接受 1024-65535 范围内的端口', () => {
        expect((window as any).ConnectionModule.isValidPort(1024)).toBe(true);
        expect((window as any).ConnectionModule.isValidPort(23456)).toBe(true);
        expect((window as any).ConnectionModule.isValidPort(65535)).toBe(true);
        expect((window as any).ConnectionModule.isValidPort(8080)).toBe(true);
    });

    test('应拒绝小于 1024 的端口', () => {
        expect((window as any).ConnectionModule.isValidPort(80)).toBe(false);
        expect((window as any).ConnectionModule.isValidPort(443)).toBe(false);
        expect((window as any).ConnectionModule.isValidPort(1023)).toBe(false);
        expect((window as any).ConnectionModule.isValidPort(0)).toBe(false);
    });

    test('应拒绝大于 65535 的端口', () => {
        expect((window as any).ConnectionModule.isValidPort(65536)).toBe(false);
        expect((window as any).ConnectionModule.isValidPort(99999)).toBe(false);
    });

    test('应拒绝非整数端口', () => {
        expect((window as any).ConnectionModule.isValidPort(NaN)).toBe(false);
        expect((window as any).ConnectionModule.isValidPort(1024.5)).toBe(false);
    });
});
