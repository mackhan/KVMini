# 关于 Tab + SDK 全功能接入 + 多语言/主题修复 实施计划

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 在设置页新增「关于」Tab，接入 tauri-commerce SDK 全部功能（反馈、分享、隐私政策、更新日志、许可状态），修复多语言/主题切换，修复显示器列表需手动刷新的问题。

**Architecture:** 在现有 4 个 Tab（通用/显示器布局/网络/安全）基础上新增第 5 个「关于」Tab，参考 OnenoteCopy 截图布局；SDK 命令已在 Rust 侧注册，前端只需补充调用逻辑；多语言/主题通过调试确认根因后修复。

**Tech Stack:** Tauri 2.x, Rust, HTML/CSS/JS, tauri-commerce SDK v0.1.2

---

### Task 1: 新增「关于」Tab HTML 结构

**Files:**
- Modify: `src/index.html`

**Step 1: 在设置页导航栏添加「关于」Tab 按钮**

在 `settings-tabs-nav` 中添加：
```html
<button class="settings-tab-btn" data-tab="about" data-i18n="tabAbout">关于</button>
```

在 `settings-tabs-select` 中添加：
```html
<option value="about" data-i18n="tabAbout">关于</option>
```

**Step 2: 添加「关于」Tab 内容区域**

在 `settings-tabs-body` 末尾（安全 Tab 之后）添加：

```html
<!-- ── 关于分页 ── -->
<div class="settings-tab-content" id="tab-about">
    <!-- 许可状态卡片（同 OnenoteCopy） -->
    <div class="settings-section">
        <h4 data-i18n="settingsLicense">许可状态</h4>
        <div id="about-trial-status" class="about-trial-status">
            <span id="about-trial-status-text" data-i18n="trialChecking">检测中...</span>
        </div>
        <div id="about-purchase-actions" style="display:none">
            <button class="btn btn-primary" id="about-purchase-btn" data-i18n="purchaseBtn">购买完整版</button>
            <button class="btn btn-outline" id="about-restore-btn" data-i18n="restoreBtn">恢复购买</button>
        </div>
    </div>

    <!-- 反馈与推荐卡片（参考截图 2x2 按钮布局） -->
    <div class="settings-section">
        <h4 data-i18n="settingsAbout">关于</h4>
        <div class="about-actions-grid">
            <button class="about-action-btn" id="about-feedback-btn">
                <span class="about-action-icon">📮</span>
                <span data-i18n="feedbackTitle">反馈与建议</span>
            </button>
            <button class="about-action-btn" id="about-share-btn">
                <span class="about-action-icon">❤️</span>
                <span data-i18n="shareTitle">推荐给朋友</span>
            </button>
            <button class="about-action-btn" id="about-privacy-btn">
                <span class="about-action-icon">🔒</span>
                <span data-i18n="privacyTitle">隐私政策</span>
            </button>
            <button class="about-action-btn" id="about-changelog-btn">
                <span class="about-action-icon">📝</span>
                <span data-i18n="changelogTitle">更新日志</span>
            </button>
        </div>
        <div class="about-rate-row">
            <button class="btn btn-outline about-rate-btn" id="about-rate-btn">
                👍 <span data-i18n="rateApp">给个赞吧</span>
            </button>
        </div>
    </div>

    <!-- 版本信息卡片 -->
    <div class="settings-section">
        <h4 data-i18n="settingsVersion">版本信息</h4>
        <div class="about-version-info">
            <span class="about-version-text" id="about-version-text">KVMini v0.1.19</span>
            <span class="about-copyright">© 2026 KVMini. All rights reserved.</span>
        </div>
    </div>
</div>
```

**Step 3: 验证 HTML 结构正确，无语法错误**

---

### Task 2: 添加「关于」Tab CSS 样式

**Files:**
- Modify: `src/styles.css`

**Step 1: 添加关于页面专用样式**

```css
/* ── 关于 Tab 样式 ── */
.about-trial-status {
    padding: 8px 12px;
    border-radius: 8px;
    background: var(--bg-secondary, #f5f5f7);
    font-size: 13px;
    color: var(--text-primary, #1d1d1f);
    margin-bottom: 8px;
}
.about-trial-status.active { color: #34c759; }
.about-trial-status.expired { color: #ff3b30; }
.about-trial-status.purchased { color: #007aff; }

.about-actions-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 8px;
    margin-bottom: 12px;
}

.about-action-btn {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 12px 16px;
    border-radius: 10px;
    border: 1px solid var(--border-color, rgba(0,0,0,0.1));
    background: var(--bg-secondary, #f5f5f7);
    cursor: pointer;
    font-size: 13px;
    color: var(--text-primary, #1d1d1f);
    transition: background 0.15s;
}
.about-action-btn:hover { background: var(--bg-hover, #e5e5ea); }
.about-action-icon { font-size: 16px; }

.about-rate-row { display: flex; justify-content: flex-end; }
.about-rate-btn { font-size: 13px; }

.about-version-info {
    display: flex;
    flex-direction: column;
    gap: 4px;
}
.about-version-text { font-size: 14px; font-weight: 500; color: var(--text-primary, #1d1d1f); }
.about-copyright { font-size: 12px; color: var(--text-secondary, #6b7280); }
```

---

### Task 3: 实现「关于」Tab JS 逻辑

**Files:**
- Modify: `src/main.js`

**Step 1: 在 `elements` 对象中添加关于 Tab 元素引用**

```js
aboutTrialStatus: document.getElementById('about-trial-status'),
aboutTrialStatusText: document.getElementById('about-trial-status-text'),
aboutPurchaseActions: document.getElementById('about-purchase-actions'),
aboutPurchaseBtn: document.getElementById('about-purchase-btn'),
aboutRestoreBtn: document.getElementById('about-restore-btn'),
aboutFeedbackBtn: document.getElementById('about-feedback-btn'),
aboutShareBtn: document.getElementById('about-share-btn'),
aboutPrivacyBtn: document.getElementById('about-privacy-btn'),
aboutChangelogBtn: document.getElementById('about-changelog-btn'),
aboutRateBtn: document.getElementById('about-rate-btn'),
aboutVersionText: document.getElementById('about-version-text'),
```

**Step 2: 添加 `initAboutTab()` 函数**

```js
async function initAboutTab() {
    // 版本号
    const meta = document.querySelector('meta[name="app-version"]');
    if (meta && elements.aboutVersionText) {
        elements.aboutVersionText.textContent = `KVMini v${meta.getAttribute('content')}`;
    }

    // 许可状态
    if (window.licenseManager) {
        const { status } = await window.licenseManager.checkLicenseStatus();
        updateAboutTrialStatus(status);
    }

    // 按钮事件
    elements.aboutFeedbackBtn?.addEventListener('click', async () => {
        try { await invoke('send_feedback'); } catch(e) { console.warn('[About] 反馈失败:', e); }
    });
    elements.aboutShareBtn?.addEventListener('click', () => {
        // 分享文本
        const text = `我在用 KVMini 跨设备共享键鼠，超好用！https://kvmini.app`;
        if (navigator.clipboard) navigator.clipboard.writeText(text);
        showSuccess('分享链接已复制到剪贴板');
    });
    elements.aboutPrivacyBtn?.addEventListener('click', () => {
        const privacy = window.__TAURI__?.shell?.open || window.open;
        // 通过 SDK 获取隐私政策内容并显示
        showPrivacyModal();
    });
    elements.aboutChangelogBtn?.addEventListener('click', () => {
        showChangelogModal();
    });
    elements.aboutRateBtn?.addEventListener('click', async () => {
        try { await invoke('request_review', { appId: 'kvmini' }); } catch(e) { console.warn('[About] 评分失败:', e); }
    });
    elements.aboutPurchaseBtn?.addEventListener('click', async () => {
        try { await invoke('purchase', { productId: 'com.kvmini.app.pro' }); } catch(e) { console.warn('[About] 购买失败:', e); }
    });
    elements.aboutRestoreBtn?.addEventListener('click', async () => {
        try { await invoke('restore_purchase'); } catch(e) { console.warn('[About] 恢复购买失败:', e); }
    });
}
```

**Step 3: 添加 `updateAboutTrialStatus(status)` 函数**

```js
function updateAboutTrialStatus(status) {
    const el = elements.aboutTrialStatus;
    const text = elements.aboutTrialStatusText;
    const actions = elements.aboutPurchaseActions;
    if (!el || !text) return;

    el.className = 'about-trial-status';
    if (status === 'licensed') {
        el.classList.add('purchased');
        text.textContent = '✓ 已购买完整版';
        if (actions) actions.style.display = 'none';
    } else if (status === 'trial') {
        el.classList.add('active');
        text.textContent = '试用中';
        if (actions) actions.style.display = 'block';
    } else if (status === 'expired') {
        el.classList.add('expired');
        text.textContent = '试用已过期';
        if (actions) actions.style.display = 'block';
    }
}
```

**Step 4: 添加 `showPrivacyModal()` 和 `showChangelogModal()` 函数**

通过 SDK 获取内容，显示在模态框中（复用现有 modal 样式）。

**Step 5: 在 `openSettings()` 中调用 `initAboutTab()`**

在 `openSettings()` 函数中，Tab 切换到 about 时调用初始化。

---

### Task 4: 修复显示器列表打开设置时自动刷新

**Files:**
- Modify: `src/main.js`

**Step 1: 在 `openSettings()` 函数中添加自动刷新**

找到 `openSettings()` 函数，在显示设置模态框后立即调用 `refreshMonitors()`：

```js
function openSettings() {
    loadSettingsToUI();
    elements.settingsModal.classList.remove('hidden');
    // 自动刷新显示器列表
    refreshMonitors();
    // ...
}
```

---

### Task 5: 调试并修复多语言/主题切换

**Files:**
- Modify: `src/main.js`
- Modify: `src/i18n.js`（如需要）

**Step 1: 在 dev 模式下打开应用，打开设置，切换语言为 English，保存**

观察控制台日志：
- 是否有 `[i18n] 语言已切换为: en` 日志
- 是否有 `applyI18n` 被调用
- DOM 中 `data-i18n` 元素是否更新

**Step 2: 检查 `applyI18n()` 函数实现**

确认 `applyI18n()` 遍历所有 `[data-i18n]` 元素并更新文本。

**Step 3: 检查主题切换**

切换主题为深色，保存，观察 `<html data-theme>` 属性是否变化。

**Step 4: 根据调试结果修复**

---

### Task 6: 添加多语言 key 支持

**Files:**
- Modify: `src/i18n.js`

**Step 1: 添加关于 Tab 相关的 i18n key**

```js
// 关于 Tab
tabAbout: '关于',
settingsLicense: '许可状态',
settingsAbout: '关于',
settingsVersion: '版本信息',
trialChecking: '检测中...',
purchaseBtn: '购买完整版',
restoreBtn: '恢复购买',
feedbackTitle: '反馈与建议',
shareTitle: '推荐给朋友',
privacyTitle: '隐私政策',
changelogTitle: '更新日志',
rateApp: '给个赞吧',
```

---

### Task 7: 版本号 +1，构建验证

**Files:**
- Modify: `package.json`
- Modify: `src-tauri/Cargo.toml`
- Modify: `src-tauri/tauri.conf.json`

**Step 1: 版本号 +1**

```bash
npm run version:update
```

**Step 2: 构建**

```bash
npm run build
```

**Step 3: 验证**

- 设置页有「关于」Tab
- 关于 Tab 显示许可状态、反馈/分享/隐私/更新日志按钮、版本信息
- 语言切换保存后立即生效
- 主题切换保存后立即生效
- 打开设置时显示器列表自动刷新

---

### Task 8: 提交代码

```bash
git add -A
git commit -m "[v0.1.20] 新增关于Tab + SDK全功能接入 + 多语言/主题修复 + 显示器自动刷新"
```
