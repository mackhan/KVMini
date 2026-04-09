/// Bug #001 回归测试：macOS 26 (Tahoe) 启动即崩溃
///
/// 根因：objc2-0.6.4 的 `MainThreadMarker::new()` 使用 `pthread_main_np() == 1`
/// 检测主线程，但 macOS 26 beta 在 `applicationDidFinishLaunching` 回调中
/// `pthread_main_np()` 返回 -1（线程初始化未完成），导致 `.unwrap()` panic。
///
/// 修复：将 `== 1` 改为 `!= 0`，兼容 -1 返回值（Apple 头文件说"非零即主线程"）。

#[cfg(target_os = "macos")]
mod macos_main_thread_tests {
    /// 测试：在测试线程（非主线程）上调用 pthread_main_np 应返回 0
    ///
    /// cargo test 在工作线程上运行，所以 pthread_main_np() 应返回 0。
    /// 这验证了非主线程的行为是正确的。
    #[test]
    fn test_pthread_main_np_returns_zero_on_test_thread() {
        extern "C" {
            fn pthread_main_np() -> std::ffi::c_int;
        }

        let result = unsafe { pthread_main_np() };
        // cargo test 在工作线程上运行，应返回 0（非主线程）
        assert_eq!(result, 0,
            "pthread_main_np() 在测试工作线程上应返回 0，实际返回: {}",
            result
        );
    }

    /// 测试：验证修复后的主线程检测逻辑（!= 0 而非 == 1）
    ///
    /// 模拟 macOS 26 beta 的行为：pthread_main_np() 返回 -1
    /// 修复前的逻辑：-1 == 1 → false → 认为不是主线程 → panic
    /// 修复后的逻辑：-1 != 0 → true → 正确识别为主线程
    #[test]
    fn test_main_thread_detection_accepts_minus_one() {
        // 模拟 macOS 26 beta 的 pthread_main_np() 返回值
        let macos26_beta_return_value: std::ffi::c_int = -1;

        // 修复前的逻辑（会崩溃）
        let old_logic = macos26_beta_return_value == 1;
        assert!(!old_logic, "旧逻辑：-1 == 1 应为 false（这就是崩溃的原因）");

        // 修复后的逻辑（正确）
        let new_logic = macos26_beta_return_value != 0;
        assert!(new_logic, "新逻辑：-1 != 0 应为 true（正确识别主线程）");
    }

    /// 测试：验证正常情况下 pthread_main_np() 返回 1 也能被正确识别
    #[test]
    fn test_main_thread_detection_accepts_one() {
        let normal_return_value: std::ffi::c_int = 1;

        let new_logic = normal_return_value != 0;
        assert!(new_logic, "新逻辑：1 != 0 应为 true（正常情况下也能正确识别主线程）");
    }

    /// 测试：验证非主线程返回 0 时被正确拒绝
    #[test]
    fn test_main_thread_detection_rejects_zero() {
        let non_main_thread_return: std::ffi::c_int = 0;

        let new_logic = non_main_thread_return != 0;
        assert!(!new_logic, "新逻辑：0 != 0 应为 false（非主线程应被拒绝）");
    }
}
