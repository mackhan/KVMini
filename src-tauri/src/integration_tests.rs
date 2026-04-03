/// 端到端集成测试
///
/// 验证完整链路：主控端捕获 → 协议编码 → TCP 发送 → 协议解码 → 被控端注入
/// 覆盖 macOS 和 Windows 坐标系、边缘切换、布局配置一致性

#[cfg(test)]
mod integration_tests {
    use crate::capturer::MockCapturer;
    use crate::client::{ClientEvent, KvmClient, ReconnectConfig};
    use crate::edge_switch::{DisplayLayout, EdgeDetector, ScreenSize, SwitchManager};
    use crate::injector::{inject_frame, MockEvent, MockInjector};
    use crate::layout_config::LayoutConfig;
    use crate::protocol::{decode, encode, KvmFrame, ScreenEdge};
    use crate::server::{KvmServer, ServerEvent};
    use crate::capturer::EventCapturer;
    use tokio::sync::mpsc;
    use tokio::time::{timeout, Duration};

    // ── 全链路测试：主控端捕获 → 编码 → TCP → 解码 → 被控端注入 ──────────

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_full_pipeline_mouse_move() {
        // 1. 启动被控端服务
        let (mut server, mut srv_rx) = KvmServer::new("127.0.0.1:0");
        let addr = server.start().await.unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;

        // 2. 启动主控端客户端
        let (mut client, mut cli_rx) = KvmClient::new(&addr.to_string());
        let frame_tx = client.start().await.unwrap();

        // 等待连接建立
        let _ = timeout(Duration::from_secs(3), cli_rx.recv()).await;
        let _ = timeout(Duration::from_secs(3), srv_rx.recv()).await;

        // 3. 模拟主控端捕获鼠标移动事件
        let capturer = MockCapturer::new();
        let (cap_tx, mut cap_rx) = mpsc::channel(16);
        capturer.start(cap_tx).unwrap();
        capturer.inject_event(KvmFrame::MouseMove { x: 5000, y: 3000 });

        // 4. 从捕获器读取事件并发送
        let frame = cap_rx.recv().await.unwrap();
        frame_tx.send(frame).await.unwrap();

        // 5. 验证被控端收到事件
        let srv_event = timeout(Duration::from_secs(2), srv_rx.recv())
            .await.unwrap().unwrap();

        assert!(matches!(
            srv_event,
            ServerEvent::InputFrame(KvmFrame::MouseMove { x: 5000, y: 3000 })
        ));

        // 6. 模拟被控端注入事件
        let injector = MockInjector::new();
        if let ServerEvent::InputFrame(f) = srv_event {
            inject_frame(&injector, &f).unwrap();
        }
        assert_eq!(injector.get_events(), vec![MockEvent::MouseMove { x: 5000, y: 3000 }]);

        capturer.stop();
        client.stop().await;
        server.stop().await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_full_pipeline_key_event() {
        let (mut server, mut srv_rx) = KvmServer::new("127.0.0.1:0");
        let addr = server.start().await.unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;

        let (mut client, mut cli_rx) = KvmClient::new(&addr.to_string());
        let frame_tx = client.start().await.unwrap();

        let _ = timeout(Duration::from_secs(3), cli_rx.recv()).await;
        let _ = timeout(Duration::from_secs(3), srv_rx.recv()).await;

        // 发送键盘事件
        frame_tx.send(KvmFrame::KeyEvent { keycode: 0x41, pressed: true }).await.unwrap();
        frame_tx.send(KvmFrame::KeyEvent { keycode: 0x41, pressed: false }).await.unwrap();

        let injector = MockInjector::new();
        for _ in 0..2 {
            let srv_event = timeout(Duration::from_secs(2), srv_rx.recv())
                .await.unwrap().unwrap();
            if let ServerEvent::InputFrame(f) = srv_event {
                inject_frame(&injector, &f).unwrap();
            }
        }

        let events = injector.get_events();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0], MockEvent::KeyEvent { keycode: 0x41, pressed: true });
        assert_eq!(events[1], MockEvent::KeyEvent { keycode: 0x41, pressed: false });

        client.stop().await;
        server.stop().await;
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_full_pipeline_switch_control() {
        let (mut server, mut srv_rx) = KvmServer::new("127.0.0.1:0");
        let addr = server.start().await.unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;

        let (mut client, mut cli_rx) = KvmClient::new(&addr.to_string());
        let frame_tx = client.start().await.unwrap();

        let _ = timeout(Duration::from_secs(3), cli_rx.recv()).await;
        let _ = timeout(Duration::from_secs(3), srv_rx.recv()).await;

        // 发送切换控制权帧
        frame_tx.send(KvmFrame::SwitchControl {
            edge: ScreenEdge::Right,
            pos_ratio: 5000,
        }).await.unwrap();

        let srv_event = timeout(Duration::from_secs(2), srv_rx.recv())
            .await.unwrap().unwrap();

        assert!(matches!(
            srv_event,
            ServerEvent::SwitchControl { edge: ScreenEdge::Right, pos_ratio: 5000 }
        ));

        client.stop().await;
        server.stop().await;
    }

    // ── 协议编解码跨平台一致性测试 ──────────────────────────────────────

    #[test]
    fn test_protocol_encode_decode_roundtrip_all_frame_types() {
        let frames = vec![
            KvmFrame::MouseMove { x: 1920, y: 1080 },
            KvmFrame::MouseButton { button: 0, pressed: true },
            KvmFrame::MouseButton { button: 1, pressed: false },
            KvmFrame::MouseScroll { dx: 0, dy: -120 },
            KvmFrame::KeyEvent { keycode: 0x41, pressed: true },
            KvmFrame::KeyEvent { keycode: 0x1B, pressed: false },
            KvmFrame::SwitchControl { edge: ScreenEdge::Left, pos_ratio: 2500 },
            KvmFrame::SwitchControl { edge: ScreenEdge::Bottom, pos_ratio: 7500 },
            KvmFrame::Heartbeat,
        ];

        for frame in &frames {
            let encoded = encode(frame);
            let (decoded, consumed) = decode(&encoded).unwrap();
            assert_eq!(&decoded, frame, "帧类型 {:?} 编解码不一致", frame);
            assert_eq!(consumed, encoded.len());
        }
    }

    #[test]
    fn test_protocol_little_endian_consistency() {
        // 验证跨平台字节序一致性（小端序）
        // 帧格式：[魔数 2B][类型 1B][flags 1B][载荷长度 4B][载荷...]
        let frame = KvmFrame::MouseMove { x: 0x1234, y: 0x5678 };
        let encoded = encode(&frame);

        // x 坐标（i32 小端序）：0x1234 = [0x34, 0x12, 0x00, 0x00]
        // 帧头 8 字节（2 魔数 + 1 类型 + 1 flags + 4 载荷长度）+ x 4 字节
        let x_bytes = &encoded[8..12];
        assert_eq!(x_bytes, &[0x34, 0x12, 0x00, 0x00], "x 坐标应为小端序");

        let y_bytes = &encoded[12..16];
        assert_eq!(y_bytes, &[0x78, 0x56, 0x00, 0x00], "y 坐标应为小端序");
    }

    // ── 边缘切换与布局配置一致性测试 ────────────────────────────────────

    #[test]
    fn test_edge_switch_layout_consistency() {
        // 验证布局配置与边缘切换逻辑的一致性
        let layouts = [
            (DisplayLayout::LocalLeftOfRemote, ScreenEdge::Right, ScreenEdge::Left),
            (DisplayLayout::LocalRightOfRemote, ScreenEdge::Left, ScreenEdge::Right),
            (DisplayLayout::LocalAboveRemote, ScreenEdge::Bottom, ScreenEdge::Top),
            (DisplayLayout::LocalBelowRemote, ScreenEdge::Top, ScreenEdge::Bottom),
        ];

        for (layout, trigger, entry) in &layouts {
            let mut mgr = SwitchManager::new(
                ScreenSize::new(1920, 1080),
                *layout,
            );

            // 验证触发边缘
            assert_eq!(layout.trigger_edge(), *trigger,
                "布局 {:?} 触发边缘应为 {:?}", layout, trigger);

            // 验证远端入口边缘
            assert_eq!(layout.remote_entry_edge(), *entry,
                "布局 {:?} 远端入口边缘应为 {:?}", layout, entry);

            // 验证 SwitchManager 行为
            let screen = ScreenSize::new(1920, 1080);
            let (trigger_x, trigger_y) = match trigger {
                ScreenEdge::Left => (0, 540),
                ScreenEdge::Right => (1919, 540),
                ScreenEdge::Top => (960, 0),
                ScreenEdge::Bottom => (960, 1079),
            };
            let action = mgr.on_mouse_move(trigger_x, trigger_y);
            assert!(
                matches!(action, crate::edge_switch::SwitchAction::SwitchToRemote { .. }),
                "布局 {:?} 在坐标 ({}, {}) 应触发切换",
                layout, trigger_x, trigger_y
            );
            let _ = screen;
        }
    }

    #[test]
    fn test_layout_config_edge_mapping_consistency() {
        // 验证 LayoutConfig 序列化后边缘映射不变
        let config = LayoutConfig {
            layout: DisplayLayout::LocalLeftOfRemote,
            local_screen_width: 1920,
            local_screen_height: 1080,
        };

        let json = serde_json::to_string(&config).unwrap();
        let restored: LayoutConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(config.layout.trigger_edge(), restored.layout.trigger_edge());
        assert_eq!(config.layout.remote_entry_edge(), restored.layout.remote_entry_edge());
    }

    // ── macOS 和 Windows 坐标系差异测试 ─────────────────────────────────

    #[test]
    fn test_coordinate_normalization_macos_1440x900() {
        // macOS Retina 屏幕（逻辑像素 1440x900）
        let detector = EdgeDetector::new(ScreenSize::new(1440, 900));

        // 右边缘中间位置
        let result = detector.detect(1439, 450);
        assert!(matches!(result, crate::edge_switch::EdgeDetectResult::Edge {
            edge: ScreenEdge::Right,
            pos_ratio: 5000,
        }));
    }

    #[test]
    fn test_coordinate_normalization_windows_1920x1080() {
        // Windows 标准屏幕（1920x1080）
        let detector = EdgeDetector::new(ScreenSize::new(1920, 1080));

        // 下边缘 75% 位置
        let result = detector.detect(1440, 1079);
        assert!(matches!(result, crate::edge_switch::EdgeDetectResult::Edge {
            edge: ScreenEdge::Bottom,
            pos_ratio: 7500,
        }));
    }

    #[test]
    fn test_coordinate_normalization_windows_4k_dpi() {
        // Windows 4K 屏幕（3840x2160，DPI 200%，逻辑像素 1920x1080）
        // 使用逻辑像素坐标
        let detector = EdgeDetector::new(ScreenSize::new(1920, 1080));
        let result = detector.detect(0, 540);
        assert!(matches!(result, crate::edge_switch::EdgeDetectResult::Edge {
            edge: ScreenEdge::Left,
            pos_ratio: 5000,
        }));
    }

    // ── 多帧连续发送测试 ─────────────────────────────────────────────────

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_full_pipeline_multiple_frames_sequence() {
        let (mut server, mut srv_rx) = KvmServer::new("127.0.0.1:0");
        let addr = server.start().await.unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;

        let (mut client, mut cli_rx) = KvmClient::new(&addr.to_string());
        let frame_tx = client.start().await.unwrap();

        let _ = timeout(Duration::from_secs(3), cli_rx.recv()).await;
        let _ = timeout(Duration::from_secs(3), srv_rx.recv()).await;

        // 发送一个完整的操作序列
        let frames = vec![
            KvmFrame::MouseMove { x: 5000, y: 5000 },
            KvmFrame::MouseButton { button: 0, pressed: true },
            KvmFrame::MouseButton { button: 0, pressed: false },
            KvmFrame::KeyEvent { keycode: 0x41, pressed: true },
            KvmFrame::KeyEvent { keycode: 0x41, pressed: false },
            KvmFrame::SwitchControl { edge: ScreenEdge::Right, pos_ratio: 5000 },
        ];

        for frame in &frames {
            frame_tx.send(frame.clone()).await.unwrap();
        }

        let injector = MockInjector::new();
        let mut received = 0;

        for _ in 0..frames.len() {
            match timeout(Duration::from_secs(2), srv_rx.recv()).await {
                Ok(Ok(ServerEvent::InputFrame(f))) => {
                    inject_frame(&injector, &f).unwrap();
                    received += 1;
                }
                Ok(Ok(ServerEvent::SwitchControl { .. })) => {
                    received += 1;
                }
                _ => break,
            }
        }

        assert_eq!(received, frames.len(), "应收到所有 {} 个事件", frames.len());

        client.stop().await;
        server.stop().await;
    }

    // ── 重连后继续发送测试 ───────────────────────────────────────────────

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn test_client_reconnect_config() {
        let (mut client, _rx) = KvmClient::new("127.0.0.1:19996");
        let mut config = ReconnectConfig::default();
        config.max_attempts = 1;
        config.initial_delay_secs = 0;
        client.set_reconnect_config(config);

        let _frame_tx = client.start().await.unwrap();
        tokio::time::sleep(Duration::from_millis(200)).await;
        client.stop().await;
    }
}
