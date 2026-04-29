import { incrementVersion, validateVersion } from '../scripts/version-update';

// 版本管理测试套件
describe('Version Manager', () => {

  // 版本号递增测试
  describe('version increment', () => {

    test('should increment patch version correctly', () => {
      expect(incrementVersion('0.1.0')).toBe('0.1.1');
      expect(incrementVersion('1.2.3')).toBe('1.2.4');
      expect(incrementVersion('2.0.9')).toBe('2.0.10');
    });

    test('should handle major and minor versions correctly', () => {
      expect(incrementVersion('1.0.0')).toBe('1.0.1');
      expect(incrementVersion('0.2.0')).toBe('0.2.1');
    });

    test('should throw error for invalid version format', () => {
      expect(() => incrementVersion('invalid')).toThrow();
      expect(() => incrementVersion('1.2')).toThrow();
      expect(() => incrementVersion('1.2.3.4')).toThrow();
    });
  });

  // 版本号校验测试
  describe('validateVersion', () => {

    test('should accept valid versions', () => {
      expect(() => validateVersion('0.1.27')).not.toThrow();
      expect(() => validateVersion('1.0.0')).not.toThrow();
      expect(() => validateVersion('10.20.30')).not.toThrow();
    });

    test('should reject invalid versions', () => {
      expect(() => validateVersion('invalid')).toThrow();
      expect(() => validateVersion('1.2')).toThrow();
      expect(() => validateVersion('v1.2.3')).toThrow();
      expect(() => validateVersion('')).toThrow();
    });
  });

  // 版本一致性测试
  describe('integration', () => {

    test('should maintain version consistency', () => {
      const startVersion = '1.2.3';
      const newVersion = incrementVersion(startVersion);

      expect(newVersion).toBe('1.2.4');
      expect(newVersion).not.toBe(startVersion);
      expect(newVersion.split('.')).toHaveLength(3);
    });

    test('should handle version rollover at patch level', () => {
      expect(incrementVersion('1.0.99')).toBe('1.0.100');
      expect(incrementVersion('0.0.0')).toBe('0.0.1');
    });
  });
});

// checkTagExists 测试（mock gh CLI）
describe('checkTagExists', () => {
  let checkTagExists: (tag: string, repo: string) => boolean;

  beforeEach(() => {
    jest.resetModules();
  });

  afterEach(() => {
    jest.restoreAllMocks();
  });

  test('should return true when gh release view succeeds (tag exists)', () => {
    jest.mock('child_process', () => ({
      execSync: jest.fn().mockReturnValue(Buffer.from('tag exists'))
    }));
    const mod = require('../scripts/version-update');
    checkTagExists = mod.checkTagExists;
    expect(checkTagExists('v0.1.28', 'mackhan/KVMini')).toBe(true);
  });

  test('should return false when gh release view fails (tag not exists)', () => {
    jest.mock('child_process', () => ({
      execSync: jest.fn().mockImplementation(() => {
        throw new Error('release not found');
      })
    }));
    const mod = require('../scripts/version-update');
    checkTagExists = mod.checkTagExists;
    expect(checkTagExists('v0.1.99', 'mackhan/KVMini')).toBe(false);
  });
});

// generate-latest-json 测试
describe('generate-latest-json', () => {
  const fs = require('fs');
  const path = require('path');
  const os = require('os');

  let generateLatestJson: (options: any) => any;
  let tmpDir: string;

  beforeEach(() => {
    tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'kvmini-test-'));
    const mod = require('../scripts/generate-latest-json');
    generateLatestJson = mod.generateLatestJson;
  });

  afterEach(() => {
    fs.rmSync(tmpDir, { recursive: true, force: true });
  });

  test('should generate latest.json with single platform entry', () => {
    const outputPath = path.join(tmpDir, 'latest.json');
    const result = generateLatestJson({
      version: '0.1.28',
      tag: 'v0.1.28',
      platformKey: 'darwin-aarch64',
      url: 'https://github.com/mackhan/KVMini/releases/download/v0.1.28/KVMini.app.tar.gz',
      signature: 'dW50cnVzdGVkIGNvbW1lbnQ=',
      outputPath,
    });

    expect(result.version).toBe('0.1.28');
    expect(result.platforms['darwin-aarch64']).toBeDefined();
    expect(result.platforms['darwin-aarch64'].url).toContain('v0.1.28');
    expect(result.platforms['darwin-aarch64'].signature).toBe('dW50cnVzdGVkIGNvbW1lbnQ=');

    // 验证文件写入
    const written = JSON.parse(fs.readFileSync(outputPath, 'utf8'));
    expect(written.version).toBe('0.1.28');
  });

  test('should merge with existing latest.json preserving other platforms', () => {
    // 先写一个已有的 latest.json（模拟 Mac 已上传）
    const existingPath = path.join(tmpDir, 'existing.json');
    fs.writeFileSync(existingPath, JSON.stringify({
      version: '0.1.28',
      notes: 'KVMini v0.1.28 release',
      pub_date: '2026-04-28T00:00:00Z',
      platforms: {
        'darwin-aarch64': {
          signature: 'mac-sig-content',
          url: 'https://github.com/mackhan/KVMini/releases/download/v0.1.28/KVMini.app.tar.gz'
        }
      }
    }));

    const outputPath = path.join(tmpDir, 'latest.json');
    const result = generateLatestJson({
      version: '0.1.28',
      tag: 'v0.1.28',
      platformKey: 'windows-x86_64',
      url: 'https://github.com/mackhan/KVMini/releases/download/v0.1.28/KVMini_0.1.28_x64-setup.nsis.zip',
      signature: 'win-sig-content',
      existingPath,
      outputPath,
    });

    // 应该同时包含两个平台
    expect(Object.keys(result.platforms)).toHaveLength(2);
    expect(result.platforms['darwin-aarch64'].signature).toBe('mac-sig-content');
    expect(result.platforms['windows-x86_64'].signature).toBe('win-sig-content');
  });

  test('should handle missing existing latest.json gracefully', () => {
    const outputPath = path.join(tmpDir, 'latest.json');
    const result = generateLatestJson({
      version: '0.1.28',
      tag: 'v0.1.28',
      platformKey: 'darwin-aarch64',
      url: 'https://example.com/test.tar.gz',
      signature: 'test-sig',
      existingPath: '/nonexistent/path/latest.json',
      outputPath,
    });

    expect(result.platforms['darwin-aarch64']).toBeDefined();
    expect(Object.keys(result.platforms)).toHaveLength(1);
  });

  test('should overwrite same platform entry on re-upload', () => {
    const existingPath = path.join(tmpDir, 'existing.json');
    fs.writeFileSync(existingPath, JSON.stringify({
      version: '0.1.28',
      notes: 'old notes',
      pub_date: '2026-04-27T00:00:00Z',
      platforms: {
        'darwin-aarch64': {
          signature: 'old-sig',
          url: 'https://example.com/old.tar.gz'
        }
      }
    }));

    const outputPath = path.join(tmpDir, 'latest.json');
    const result = generateLatestJson({
      version: '0.1.28',
      tag: 'v0.1.28',
      platformKey: 'darwin-aarch64',
      url: 'https://example.com/new.tar.gz',
      signature: 'new-sig',
      existingPath,
      outputPath,
    });

    // 同平台应该被覆盖为新值
    expect(result.platforms['darwin-aarch64'].signature).toBe('new-sig');
    expect(result.platforms['darwin-aarch64'].url).toBe('https://example.com/new.tar.gz');
    expect(Object.keys(result.platforms)).toHaveLength(1);
  });
});
