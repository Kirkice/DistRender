//! 事件系统演示
//!
//! 演示如何使用 DistRender 的事件系统处理窗口、鼠标和键盘事件。
//!
//! # 运行方式
//!
//! ```bash
//! cargo run --example event_system_demo
//! ```

use dist_render::core::event::*;

fn main() {
    println!("=== DistRender 事件系统演示 ===\n");

    // === 1. 演示窗口调整大小事件 ===
    println!("--- 1. 窗口调整大小事件 ---");
    let mut resize_event = WindowResizeEvent::new(1920, 1080);
    let mut dispatcher = EventDispatcher::new(&mut resize_event);

    dispatcher.dispatch(EventType::WindowResize, |e| {
        println!("✓ 处理事件: {}", e.detail());
        true
    });

    println!("事件处理状态: {}\n", resize_event.is_handled());

    // === 2. 演示鼠标事件 ===
    println!("--- 2. 鼠标事件 ---");

    // 鼠标按下
    let mut mouse_down = MouseButtonEvent::pressed(MouseButton::Left, 100.0, 200.0);
    println!("事件类型: {:?}", mouse_down.event_type());
    println!("事件详情: {}\n", mouse_down.detail());

    // 鼠标释放
    let mut mouse_up = MouseButtonEvent::released(MouseButton::Right, 150.0, 250.0);
    println!("事件类型: {:?}", mouse_up.event_type());
    println!("事件详情: {}\n", mouse_up.detail());

    // 鼠标移动
    let mut mouse_move = MouseMoveEvent::new(300.0, 400.0, 10.0, 20.0);
    println!("事件类型: {:?}", mouse_move.event_type());
    println!("事件详情: {}\n", mouse_move.detail());

    // === 3. 演示键盘事件 ===
    println!("--- 3. 键盘事件 ---");

    let keys = vec![
        (KeyCode::W, "前进"),
        (KeyCode::A, "左移"),
        (KeyCode::S, "后退"),
        (KeyCode::D, "右移"),
        (KeyCode::Space, "跳跃"),
        (KeyCode::Escape, "退出"),
    ];

    for (key, desc) in keys {
        let mut key_event = KeyboardEvent::pressed(key);
        println!("{}: {:?} - {}", desc, key, key_event.detail());
    }

    // === 4. 演示事件分发器的类型检查 ===
    println!("\n--- 4. 事件分发器类型检查 ---");
    let mut window_event = WindowResizeEvent::new(800, 600);
    let mut dispatcher = EventDispatcher::new(&mut window_event);

    // 尝试用错误的事件类型分发（不会被调用）
    let handled = dispatcher.dispatch(EventType::MouseButtonDown, |_| {
        println!("❌ 这不应该被打印（类型不匹配）");
        true
    });
    println!("错误类型分发结果: {} (预期为 false)", handled);

    // 使用正确的事件类型分发
    let handled = dispatcher.dispatch(EventType::WindowResize, |e| {
        println!("✓ 正确处理: {}", e.detail());
        true
    });
    println!("正确类型分发结果: {} (预期为 true)", handled);

    // === 5. 演示时钟和绘制事件 ===
    println!("\n--- 5. 时钟和绘制事件 ---");

    let tick_event = TickEvent::new(0.016, 1.5);
    println!("时钟事件: {}", tick_event.detail());

    let draw_event = DrawEvent::new();
    println!("绘制事件: {}", draw_event.detail());

    // === 6. 演示事件处理链 ===
    println!("\n--- 6. 事件处理链 ---");
    let mut key_event = KeyboardEvent::pressed(KeyCode::Escape);
    let mut dispatcher = EventDispatcher::new(&mut key_event);

    // 第一个处理器
    dispatcher.dispatch(EventType::KeyDown, |e| {
        println!("处理器 1: {}", e.detail());
        false // 不标记为已处理，继续传递
    });

    // 检查处理状态
    if !dispatcher.is_handled() {
        println!("事件未被处理，可以继续传递给其他处理器");

        // 第二个处理器
        dispatcher.dispatch(EventType::KeyDown, |e| {
            println!("处理器 2: {}", e.detail());
            true // 标记为已处理
        });
    }

    println!("最终处理状态: {}", dispatcher.is_handled());

    println!("\n=== 演示完成 ===");
    println!("\n事件系统特性总结:");
    println!("✓ 类型安全的事件分发");
    println!("✓ 事件处理状态追踪");
    println!("✓ 支持多种事件类型（窗口、鼠标、键盘、时钟、绘制）");
    println!("✓ 事件处理链支持");
    println!("✓ 零成本抽象设计");
}
