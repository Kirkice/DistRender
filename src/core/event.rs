//! 事件系统模块
//!
//! 提供统一的事件处理机制，参考 DistEngine 的 Event/EventDispatcher 设计。
//! 支持窗口事件、鼠标事件、键盘事件等。
//!
//! # 设计原则
//!
//! - **类型安全**：使用 Rust 的类型系统保证事件处理的安全性
//! - **可扩展性**：易于添加新的事件类型
//! - **高性能**：使用零成本抽象，避免运行时开销
//! - **清晰的语义**：事件类型和处理逻辑一目了然
//!
//! # 与 DistEngine 的对比
//!
//! | 特性 | DistEngine (C++) | DistRender (Rust) |
//! |------|------------------|-------------------|
//! | 事件基类 | `Event` 抽象类 | `Event` trait |
//! | 类型标识 | 运行时多态 | 编译时枚举 |
//! | 分发机制 | 模板 + dynamic_cast | trait + 模式匹配 |
//! | 性能 | 虚函数调用开销 | 零成本抽象 |
//!
//! # 使用示例

// 允许未使用的代码，事件系统将在未来版本中使用
#![allow(dead_code)]
//!
//! ```no_run
//! use DistRender::core::event::*;
//!
//! // 1. 创建事件
//! let mut event = WindowResizeEvent::new(1920, 1080);
//!
//! // 2. 创建分发器
//! let mut dispatcher = EventDispatcher::new(&mut event);
//!
//! // 3. 分发事件到处理函数
//! dispatcher.dispatch(EventType::WindowResize, |e| {
//!     println!("窗口调整为: {}", e.detail());
//!     true // 返回 true 表示事件已处理
//! });
//!
//! // 4. 检查处理状态
//! if dispatcher.is_handled() {
//!     println!("事件已被成功处理");
//! }
//! ```
//!
//! # 事件处理链
//!
//! 支持多个处理器按顺序处理同一事件，类似责任链模式：
//!
//! ```no_run
//! use DistRender::core::event::*;
//!
//! let mut event = KeyboardEvent::pressed(KeyCode::Escape);
//! let mut dispatcher = EventDispatcher::new(&mut event);
//!
//! // 第一个处理器
//! dispatcher.dispatch(EventType::KeyDown, |e| {
//!     println!("处理器 1: {}", e.detail());
//!     false // 不标记为已处理，继续传递
//! });
//!
//! // 如果未被处理，传递给下一个处理器
//! if !dispatcher.is_handled() {
//!     dispatcher.dispatch(EventType::KeyDown, |e| {
//!         println!("处理器 2: {}", e.detail());
//!         true // 标记为已处理，停止传递
//!     });
//! }
//! ```

use std::fmt;

/// 事件类型枚举
///
/// 定义系统中所有支持的事件类型，参考 DistEngine 的 EventType。
///
/// # 设计说明
///
/// 使用枚举而非 C++ 中的整数常量，提供编译时类型检查。
/// 每个变体对应一种具体的事件类型，避免了类型转换错误。
///
/// # 派生特性
///
/// - `Debug`: 支持调试输出
/// - `Clone, Copy`: 轻量级复制，适合作为标识符
/// - `PartialEq, Eq`: 支持相等性比较
/// - `Hash`: 支持作为哈希表的键（用于事件订阅系统）
///
/// # 示例
///
/// ```
/// use DistRender::core::event::EventType;
///
/// let event_type = EventType::WindowResize;
/// println!("事件名称: {}", event_type.name());
///
/// // 可以用于模式匹配
/// match event_type {
///     EventType::WindowResize => println!("处理窗口调整"),
///     EventType::MouseButtonDown => println!("处理鼠标点击"),
///     _ => println!("其他事件"),
/// }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EventType {
    /// 空事件
    ///
    /// 用作占位符或初始值，实际不应被分发
    None,

    /// 窗口调整大小事件
    ///
    /// 当用户拖动窗口边缘或最大化/还原窗口时触发
    WindowResize,

    /// 窗口关闭事件
    ///
    /// 当用户点击关闭按钮或按 Alt+F4 时触发
    WindowClose,

    /// 鼠标按下事件
    ///
    /// 当鼠标按钮被按下时触发（包括左键、右键、中键）
    MouseButtonDown,

    /// 鼠标释放事件
    ///
    /// 当鼠标按钮被释放时触发
    MouseButtonUp,

    /// 鼠标移动事件
    ///
    /// 当鼠标在窗口内移动时触发，包含位置和增量信息
    MouseMove,

    /// 鼠标滚轮事件
    ///
    /// 当用户滚动鼠标滚轮或触摸板手势时触发
    MouseScroll,

    /// 键盘按下事件
    ///
    /// 当键盘按键被按下时触发
    KeyDown,

    /// 键盘释放事件
    ///
    /// 当键盘按键被释放时触发
    KeyUp,

    /// 时钟事件（每帧）
    ///
    /// 在游戏循环中每帧触发，用于更新游戏逻辑
    Tick,

    /// 绘制事件
    ///
    /// 通知渲染器执行绘制操作
    Draw,
}

impl EventType {
    /// 获取事件类型的名称
    ///
    /// 返回事件类型的字符串表示，主要用于日志记录和调试。
    ///
    /// # 返回值
    ///
    /// 返回静态字符串切片，不涉及堆分配，性能开销为零。
    ///
    /// # 示例
    ///
    /// ```
    /// use DistRender::core::event::EventType;
    ///
    /// let event_type = EventType::MouseButtonDown;
    /// assert_eq!(event_type.name(), "MouseButtonDown");
    ///
    /// // 用于日志记录
    /// println!("收到事件: {}", event_type.name());
    /// ```
    pub fn name(&self) -> &'static str {
        match self {
            EventType::None => "None",
            EventType::WindowResize => "WindowResize",
            EventType::WindowClose => "WindowClose",
            EventType::MouseButtonDown => "MouseButtonDown",
            EventType::MouseButtonUp => "MouseButtonUp",
            EventType::MouseMove => "MouseMove",
            EventType::MouseScroll => "MouseScroll",
            EventType::KeyDown => "KeyDown",
            EventType::KeyUp => "KeyUp",
            EventType::Tick => "Tick",
            EventType::Draw => "Draw",
        }
    }
}

/// 鼠标按钮枚举
///
/// 表示鼠标的各种按钮，支持标准的三键鼠标以及额外的按钮。
///
/// # 示例
///
/// ```
/// use DistRender::core::event::MouseButton;
///
/// let left_button = MouseButton::Left;
/// let side_button = MouseButton::Other(3); // 侧键
///
/// // 用于模式匹配
/// match left_button {
///     MouseButton::Left => println!("左键操作"),
///     MouseButton::Right => println!("右键菜单"),
///     MouseButton::Middle => println!("中键滚动"),
///     MouseButton::Other(n) => println!("额外按钮 {}", n),
/// }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MouseButton {
    /// 左键（主按钮）
    ///
    /// 通常用于选择、点击、拖拽等主要交互
    Left,

    /// 右键（次按钮）
    ///
    /// 通常用于打开上下文菜单
    Right,

    /// 中键（滚轮按钮）
    ///
    /// 通常用于打开链接到新标签页或平移视图
    Middle,

    /// 其他按钮
    ///
    /// 用于支持多键鼠标的额外按钮（如侧键）
    /// 参数为按钮编号，通常从 3 开始
    Other(u8),
}

/// 键盘按键枚举（简化版本）
///
/// 目前仅包含常用按键，可根据需要扩展。
/// 对于未列出的按键，使用 `Other` 变体。
///
/// # 设计说明
///
/// 这是一个简化版本，仅包含游戏和应用程序中最常用的按键。
/// 完整的键盘支持可以考虑使用 `winit` 或 `sdl2` 等库的按键码。
///
/// # 示例
///
/// ```
/// use DistRender::core::event::KeyCode;
///
/// // WASD 移动控制
/// let key = KeyCode::W;
/// match key {
///     KeyCode::W => println!("向前移动"),
///     KeyCode::A => println!("向左移动"),
///     KeyCode::S => println!("向后移动"),
///     KeyCode::D => println!("向右移动"),
///     KeyCode::Space => println!("跳跃"),
///     _ => println!("其他按键"),
/// }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyCode {
    /// W 键
    ///
    /// 常用于向前移动
    W,

    /// A 键
    ///
    /// 常用于向左移动
    A,

    /// S 键
    ///
    /// 常用于向后移动
    S,

    /// D 键
    ///
    /// 常用于向右移动
    D,

    /// 空格键
    ///
    /// 常用于跳跃或确认
    Space,

    /// Escape 键
    ///
    /// 常用于取消或打开菜单
    Escape,

    /// Enter 键（回车键）
    ///
    /// 常用于确认或换行
    Enter,

    /// F1-F12 功能键
    ///
    /// 通常用于快捷操作或调试功能
    F1, F2, F3, F4, F5, F6, F7, F8, F9, F10, F11, F12,

    /// 其他按键
    ///
    /// 用于未明确列出的按键
    /// 参数为平台相关的虚拟键码
    Other(u32),
}

/// 事件 trait
///
/// 所有事件都必须实现此 trait，类似 DistEngine 的 Event 基类。
///
/// # 设计说明
///
/// - **Trait 而非继承**：Rust 使用 trait 替代 C++ 的虚继承
/// - **必须是 Debug**：所有事件必须支持调试输出
/// - **处理状态追踪**：通过 `is_handled()` 支持事件处理链
///
/// # 实现要求
///
/// 实现此 trait 的类型需要：
/// 1. 实现 `Debug` trait（通过 `#[derive(Debug)]` 自动实现）
/// 2. 提供 `event_type()` 方法返回事件类型
/// 3. 提供 `is_handled()` 和 `set_handled()` 方法管理处理状态
/// 4. 可选：覆盖 `detail()` 方法提供自定义的详细信息
///
/// # 示例
///
/// ```
/// use DistRender::core::event::{Event, EventType};
///
/// #[derive(Debug)]
/// struct MyCustomEvent {
///     data: String,
///     handled: bool,
/// }
///
/// impl Event for MyCustomEvent {
///     fn event_type(&self) -> EventType {
///         EventType::None // 或自定义类型
///     }
///
///     fn detail(&self) -> String {
///         format!("MyCustomEvent: {}", self.data)
///     }
///
///     fn is_handled(&self) -> bool {
///         self.handled
///     }
///
///     fn set_handled(&mut self, handled: bool) {
///         self.handled = handled;
///     }
/// }
/// ```
pub trait Event: fmt::Debug {
    /// 获取事件类型
    ///
    /// 返回此事件的类型标识，用于事件分发器进行类型检查。
    ///
    /// # 返回值
    ///
    /// 返回 `EventType` 枚举值
    fn event_type(&self) -> EventType;

    /// 获取事件详细信息（用于调试和日志）
    ///
    /// 提供人类可读的事件描述，默认实现使用 `Debug` trait。
    /// 具体事件类型可以覆盖此方法提供更友好的输出。
    ///
    /// # 返回值
    ///
    /// 返回描述事件的字符串
    ///
    /// # 示例
    ///
    /// ```
    /// use DistRender::core::event::{Event, WindowResizeEvent};
    ///
    /// let event = WindowResizeEvent::new(1920, 1080);
    /// println!("{}", event.detail()); // 输出: WindowResize: 1920x1080
    /// ```
    fn detail(&self) -> String {
        format!("{:?}", self)
    }

    /// 事件是否已被处理
    ///
    /// 用于实现事件处理链，允许多个处理器按顺序处理事件。
    /// 如果返回 `true`，表示事件已被处理，通常不应继续传递。
    ///
    /// # 返回值
    ///
    /// - `true`: 事件已被处理
    /// - `false`: 事件尚未被处理
    fn is_handled(&self) -> bool;

    /// 设置事件处理状态
    ///
    /// 标记事件为已处理或未处理。通常在事件处理函数中调用。
    ///
    /// # 参数
    ///
    /// * `handled` - `true` 表示已处理，`false` 表示未处理
    ///
    /// # 示例
    ///
    /// ```
    /// use DistRender::core::event::{Event, WindowResizeEvent};
    ///
    /// let mut event = WindowResizeEvent::new(800, 600);
    /// assert!(!event.is_handled());
    ///
    /// event.set_handled(true);
    /// assert!(event.is_handled());
    /// ```
    fn set_handled(&mut self, handled: bool);
}

/// 窗口调整大小事件
///
/// 当窗口尺寸改变时触发，包含新的窗口宽度和高度。
///
/// # 使用场景
///
/// - 调整渲染视口大小
/// - 重新创建交换链（Vulkan/DX12）
/// - 更新投影矩阵的宽高比
/// - 重新布局 UI 元素
///
/// # 示例
///
/// ```
/// use DistRender::core::event::{Event, EventType, EventDispatcher, WindowResizeEvent};
///
/// let mut event = WindowResizeEvent::new(1920, 1080);
/// let mut dispatcher = EventDispatcher::new(&mut event);
///
/// dispatcher.dispatch(EventType::WindowResize, |e| {
///     // 在这里处理窗口调整
///     println!("窗口调整为: {}", e.detail());
///     true
/// });
/// ```
#[derive(Debug, Clone)]
pub struct WindowResizeEvent {
    /// 新的窗口宽度（像素）
    ///
    /// 表示窗口客户区域的宽度，不包括边框和标题栏
    pub width: u32,

    /// 新的窗口高度（像素）
    ///
    /// 表示窗口客户区域的高度，不包括边框和标题栏
    pub height: u32,

    /// 事件是否已处理
    ///
    /// 用于事件处理链，标记此事件是否已被某个处理器处理
    handled: bool,
}

impl WindowResizeEvent {
    /// 创建新的窗口调整大小事件
    ///
    /// # 参数
    ///
    /// * `width` - 新的窗口宽度（像素）
    /// * `height` - 新的窗口高度（像素）
    ///
    /// # 返回值
    ///
    /// 返回新创建的窗口调整大小事件，初始处理状态为 `false`
    ///
    /// # 示例
    ///
    /// ```
    /// use DistRender::core::event::WindowResizeEvent;
    ///
    /// let event = WindowResizeEvent::new(1920, 1080);
    /// assert_eq!(event.width, 1920);
    /// assert_eq!(event.height, 1080);
    /// ```
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            handled: false,
        }
    }
}

impl Event for WindowResizeEvent {
    fn event_type(&self) -> EventType {
        EventType::WindowResize
    }

    fn detail(&self) -> String {
        format!("WindowResize: {}x{}", self.width, self.height)
    }

    fn is_handled(&self) -> bool {
        self.handled
    }

    fn set_handled(&mut self, handled: bool) {
        self.handled = handled;
    }
}

/// 窗口关闭事件
#[derive(Debug, Clone)]
pub struct WindowCloseEvent {
    /// 事件是否已处理
    handled: bool,
}

impl WindowCloseEvent {
    /// 创建新的窗口关闭事件
    pub fn new() -> Self {
        Self { handled: false }
    }
}

impl Default for WindowCloseEvent {
    fn default() -> Self {
        Self::new()
    }
}

impl Event for WindowCloseEvent {
    fn event_type(&self) -> EventType {
        EventType::WindowClose
    }

    fn detail(&self) -> String {
        "WindowClose".to_string()
    }

    fn is_handled(&self) -> bool {
        self.handled
    }

    fn set_handled(&mut self, handled: bool) {
        self.handled = handled;
    }
}

/// 鼠标按钮事件
///
/// 表示鼠标按钮的按下或释放，包含按钮类型和点击位置。
///
/// # 使用场景
///
/// - 处理点击交互（选择、拖拽等）
/// - 实现上下文菜单（右键）
/// - 检测双击或长按
/// - UI 元素的交互响应
///
/// # 示例
///
/// ```
/// use DistRender::core::event::{MouseButtonEvent, MouseButton};
///
/// // 鼠标按下
/// let press_event = MouseButtonEvent::pressed(MouseButton::Left, 100.0, 200.0);
/// println!("左键按下于: ({}, {})", press_event.x, press_event.y);
///
/// // 鼠标释放
/// let release_event = MouseButtonEvent::released(MouseButton::Right, 150.0, 250.0);
/// println!("右键释放于: ({}, {})", release_event.x, release_event.y);
/// ```
#[derive(Debug, Clone)]
pub struct MouseButtonEvent {
    /// 鼠标按钮类型
    ///
    /// 指示哪个鼠标按钮触发了此事件（左键、右键、中键等）
    pub button: MouseButton,

    /// 鼠标 X 坐标（像素）
    ///
    /// 相对于窗口客户区左上角的水平位置
    pub x: f32,

    /// 鼠标 Y 坐标（像素）
    ///
    /// 相对于窗口客户区左上角的垂直位置
    pub y: f32,

    /// 是否是按下事件
    ///
    /// - `true`: 按钮被按下（MouseButtonDown）
    /// - `false`: 按钮被释放（MouseButtonUp）
    pub pressed: bool,

    /// 事件是否已处理
    handled: bool,
}

impl MouseButtonEvent {
    /// 创建鼠标按下事件
    ///
    /// # 参数
    ///
    /// * `button` - 被按下的鼠标按钮
    /// * `x` - 鼠标 X 坐标
    /// * `y` - 鼠标 Y 坐标
    ///
    /// # 返回值
    ///
    /// 返回新创建的鼠标按下事件
    ///
    /// # 示例
    ///
    /// ```
    /// use DistRender::core::event::{MouseButtonEvent, MouseButton};
    ///
    /// let event = MouseButtonEvent::pressed(MouseButton::Left, 100.0, 200.0);
    /// assert!(event.pressed);
    /// assert_eq!(event.button, MouseButton::Left);
    /// ```
    pub fn pressed(button: MouseButton, x: f32, y: f32) -> Self {
        Self {
            button,
            x,
            y,
            pressed: true,
            handled: false,
        }
    }

    /// 创建鼠标释放事件
    ///
    /// # 参数
    ///
    /// * `button` - 被释放的鼠标按钮
    /// * `x` - 鼠标 X 坐标
    /// * `y` - 鼠标 Y 坐标
    ///
    /// # 返回值
    ///
    /// 返回新创建的鼠标释放事件
    ///
    /// # 示例
    ///
    /// ```
    /// use DistRender::core::event::{MouseButtonEvent, MouseButton};
    ///
    /// let event = MouseButtonEvent::released(MouseButton::Right, 150.0, 250.0);
    /// assert!(!event.pressed);
    /// assert_eq!(event.button, MouseButton::Right);
    /// ```
    pub fn released(button: MouseButton, x: f32, y: f32) -> Self {
        Self {
            button,
            x,
            y,
            pressed: false,
            handled: false,
        }
    }
}

impl Event for MouseButtonEvent {
    fn event_type(&self) -> EventType {
        if self.pressed {
            EventType::MouseButtonDown
        } else {
            EventType::MouseButtonUp
        }
    }

    fn detail(&self) -> String {
        format!(
            "MouseButton{}: {:?} at ({}, {})",
            if self.pressed { "Down" } else { "Up" },
            self.button,
            self.x,
            self.y
        )
    }

    fn is_handled(&self) -> bool {
        self.handled
    }

    fn set_handled(&mut self, handled: bool) {
        self.handled = handled;
    }
}

/// 鼠标移动事件
///
/// 当鼠标在窗口内移动时触发，包含当前位置和移动增量。
///
/// # 使用场景
///
/// - 实现鼠标悬停效果
/// - 第一人称相机控制（使用 delta）
/// - 拖拽操作
/// - 绘画应用的笔刷跟踪
///
/// # 性能考虑
///
/// 鼠标移动事件可能会非常频繁（每帧多次），在处理时应注意性能。
/// 考虑使用增量值而非绝对位置进行相机旋转等操作。
///
/// # 示例
///
/// ```
/// use DistRender::core::event::MouseMoveEvent;
///
/// // 创建鼠标移动事件
/// let event = MouseMoveEvent::new(150.0, 200.0, 10.0, -5.0);
///
/// // 使用绝对位置
/// println!("鼠标位置: ({}, {})", event.x, event.y);
///
/// // 使用增量进行相机旋转
/// let sensitivity = 0.1;
/// let yaw = event.delta_x * sensitivity;
/// let pitch = event.delta_y * sensitivity;
/// ```
#[derive(Debug, Clone)]
pub struct MouseMoveEvent {
    /// 鼠标 X 坐标（像素）
    ///
    /// 相对于窗口客户区左上角的当前水平位置
    pub x: f32,

    /// 鼠标 Y 坐标（像素）
    ///
    /// 相对于窗口客户区左上角的当前垂直位置
    pub y: f32,

    /// X 方向移动增量（像素）
    ///
    /// 自上次鼠标移动事件以来的水平位移
    /// - 正值：向右移动
    /// - 负值：向左移动
    pub delta_x: f32,

    /// Y 方向移动增量（像素）
    ///
    /// 自上次鼠标移动事件以来的垂直位移
    /// - 正值：向下移动
    /// - 负值：向上移动
    pub delta_y: f32,

    /// 事件是否已处理
    handled: bool,
}

impl MouseMoveEvent {
    /// 创建新的鼠标移动事件
    ///
    /// # 参数
    ///
    /// * `x` - 当前鼠标 X 坐标
    /// * `y` - 当前鼠标 Y 坐标
    /// * `delta_x` - X 方向移动增量
    /// * `delta_y` - Y 方向移动增量
    ///
    /// # 返回值
    ///
    /// 返回新创建的鼠标移动事件
    ///
    /// # 示例
    ///
    /// ```
    /// use DistRender::core::event::MouseMoveEvent;
    ///
    /// let event = MouseMoveEvent::new(300.0, 400.0, 10.0, 20.0);
    /// assert_eq!(event.x, 300.0);
    /// assert_eq!(event.delta_x, 10.0);
    /// ```
    pub fn new(x: f32, y: f32, delta_x: f32, delta_y: f32) -> Self {
        Self {
            x,
            y,
            delta_x,
            delta_y,
            handled: false,
        }
    }
}

impl Event for MouseMoveEvent {
    fn event_type(&self) -> EventType {
        EventType::MouseMove
    }

    fn detail(&self) -> String {
        format!(
            "MouseMove: ({}, {}) delta: ({}, {})",
            self.x, self.y, self.delta_x, self.delta_y
        )
    }

    fn is_handled(&self) -> bool {
        self.handled
    }

    fn set_handled(&mut self, handled: bool) {
        self.handled = handled;
    }
}

/// 鼠标滚轮事件
#[derive(Debug, Clone)]
pub struct MouseScrollEvent {
    /// 水平滚动增量
    pub delta_x: f32,
    /// 垂直滚动增量
    pub delta_y: f32,
    /// 事件是否已处理
    handled: bool,
}

impl MouseScrollEvent {
    /// 创建新的鼠标滚轮事件
    pub fn new(delta_x: f32, delta_y: f32) -> Self {
        Self {
            delta_x,
            delta_y,
            handled: false,
        }
    }
}

impl Event for MouseScrollEvent {
    fn event_type(&self) -> EventType {
        EventType::MouseScroll
    }

    fn detail(&self) -> String {
        format!("MouseScroll: delta ({}, {})", self.delta_x, self.delta_y)
    }

    fn is_handled(&self) -> bool {
        self.handled
    }

    fn set_handled(&mut self, handled: bool) {
        self.handled = handled;
    }
}

/// 键盘事件
///
/// 表示键盘按键的按下或释放。
///
/// # 使用场景
///
/// - 游戏角色移动控制（WASD）
/// - 快捷键处理
/// - 文本输入（需要额外的字符事件支持）
/// - 菜单导航
///
/// # 设计说明
///
/// 此事件仅处理物理按键状态，不包含字符输入信息。
/// 对于文本输入，应使用专门的字符输入事件（未来可能添加）。
///
/// # 示例
///
/// ```
/// use DistRender::core::event::{Event, EventType, EventDispatcher, KeyboardEvent, KeyCode};
///
/// // WASD 移动控制示例
/// let mut event = KeyboardEvent::pressed(KeyCode::W);
/// let mut dispatcher = EventDispatcher::new(&mut event);
///
/// dispatcher.dispatch(EventType::KeyDown, |e| {
///     println!("按键按下: {}", e.detail());
///     true
/// });
/// ```
#[derive(Debug, Clone)]
pub struct KeyboardEvent {
    /// 按键码
    ///
    /// 标识具体按下或释放的键盘按键
    pub key_code: KeyCode,

    /// 是否是按下事件
    ///
    /// - `true`: 按键被按下（KeyDown）
    /// - `false`: 按键被释放（KeyUp）
    pub pressed: bool,

    /// 事件是否已处理
    handled: bool,
}

impl KeyboardEvent {
    /// 创建按键按下事件
    ///
    /// # 参数
    ///
    /// * `key_code` - 被按下的按键码
    ///
    /// # 返回值
    ///
    /// 返回新创建的按键按下事件
    ///
    /// # 示例
    ///
    /// ```
    /// use DistRender::core::event::{KeyboardEvent, KeyCode};
    ///
    /// let event = KeyboardEvent::pressed(KeyCode::Space);
    /// assert!(event.pressed);
    /// assert_eq!(event.key_code, KeyCode::Space);
    /// ```
    pub fn pressed(key_code: KeyCode) -> Self {
        Self {
            key_code,
            pressed: true,
            handled: false,
        }
    }

    /// 创建按键释放事件
    ///
    /// # 参数
    ///
    /// * `key_code` - 被释放的按键码
    ///
    /// # 返回值
    ///
    /// 返回新创建的按键释放事件
    ///
    /// # 示例
    ///
    /// ```
    /// use DistRender::core::event::{KeyboardEvent, KeyCode};
    ///
    /// let event = KeyboardEvent::released(KeyCode::Escape);
    /// assert!(!event.pressed);
    /// assert_eq!(event.key_code, KeyCode::Escape);
    /// ```
    pub fn released(key_code: KeyCode) -> Self {
        Self {
            key_code,
            pressed: false,
            handled: false,
        }
    }
}

impl Event for KeyboardEvent {
    fn event_type(&self) -> EventType {
        if self.pressed {
            EventType::KeyDown
        } else {
            EventType::KeyUp
        }
    }

    fn detail(&self) -> String {
        format!(
            "Key{}: {:?}",
            if self.pressed { "Down" } else { "Up" },
            self.key_code
        )
    }

    fn is_handled(&self) -> bool {
        self.handled
    }

    fn set_handled(&mut self, handled: bool) {
        self.handled = handled;
    }
}

/// 时钟事件（每帧）
///
/// 在游戏循环中每帧触发，用于更新游戏逻辑和动画。
///
/// # 使用场景
///
/// - 更新游戏对象的位置和状态
/// - 播放动画
/// - 物理模拟
/// - AI 更新
/// - 粒子系统更新
///
/// # 设计说明
///
/// 使用 `delta_time` 进行帧率无关的更新，确保在不同帧率下
/// 游戏逻辑的表现一致。
///
/// # 示例
///
/// ```
/// use DistRender::core::event::{Event, EventType, EventDispatcher, TickEvent};
///
/// // 模拟 60 FPS 的一帧
/// let mut event = TickEvent::new(0.016, 1.5);
/// let mut dispatcher = EventDispatcher::new(&mut event);
///
/// dispatcher.dispatch(EventType::Tick, |e| {
///     // 更新游戏逻辑
///     // let dt = event.delta_time;
///     // player.position += player.velocity * dt;
///     true
/// });
/// ```
///
/// # 帧率无关的移动示例
///
/// ```
/// # use DistRender::core::event::TickEvent;
/// let event = TickEvent::new(0.016, 1.5);
///
/// // 速度：每秒 100 像素
/// let velocity = 100.0;
///
/// // 帧率无关的位移计算
/// let displacement = velocity * event.delta_time;
/// // 在 60 FPS 下约为 1.6 像素/帧
/// ```
#[derive(Debug, Clone)]
pub struct TickEvent {
    /// 帧间隔时间（秒）
    ///
    /// 自上一帧以来经过的时间，通常称为 delta time 或 dt。
    /// 使用此值进行帧率无关的更新计算。
    ///
    /// 典型值：
    /// - 60 FPS: ~0.0166 秒
    /// - 30 FPS: ~0.0333 秒
    /// - 144 FPS: ~0.0069 秒
    pub delta_time: f32,

    /// 总运行时间（秒）
    ///
    /// 自应用程序启动以来经过的总时间。
    /// 可用于基于时间的动画和周期性效果。
    pub total_time: f32,

    /// 事件是否已处理
    handled: bool,
}

impl TickEvent {
    /// 创建新的时钟事件
    ///
    /// # 参数
    ///
    /// * `delta_time` - 帧间隔时间（秒）
    /// * `total_time` - 总运行时间（秒）
    ///
    /// # 返回值
    ///
    /// 返回新创建的时钟事件
    ///
    /// # 示例
    ///
    /// ```
    /// use DistRender::core::event::TickEvent;
    ///
    /// // 模拟 60 FPS
    /// let event = TickEvent::new(1.0 / 60.0, 5.0);
    /// assert!((event.delta_time - 0.0166).abs() < 0.001);
    /// assert_eq!(event.total_time, 5.0);
    /// ```
    pub fn new(delta_time: f32, total_time: f32) -> Self {
        Self {
            delta_time,
            total_time,
            handled: false,
        }
    }
}

impl Event for TickEvent {
    fn event_type(&self) -> EventType {
        EventType::Tick
    }

    fn detail(&self) -> String {
        format!(
            "Tick: dt={:.3}s, total={:.2}s",
            self.delta_time, self.total_time
        )
    }

    fn is_handled(&self) -> bool {
        self.handled
    }

    fn set_handled(&mut self, handled: bool) {
        self.handled = handled;
    }
}

/// 绘制事件
#[derive(Debug, Clone)]
pub struct DrawEvent {
    /// 事件是否已处理
    handled: bool,
}

impl DrawEvent {
    /// 创建新的绘制事件
    pub fn new() -> Self {
        Self { handled: false }
    }
}

impl Default for DrawEvent {
    fn default() -> Self {
        Self::new()
    }
}

impl Event for DrawEvent {
    fn event_type(&self) -> EventType {
        EventType::Draw
    }

    fn detail(&self) -> String {
        "Draw".to_string()
    }

    fn is_handled(&self) -> bool {
        self.handled
    }

    fn set_handled(&mut self, handled: bool) {
        self.handled = handled;
    }
}

/// 事件分发器
///
/// 类似 DistEngine 的 EventDispatcher，用于将事件分发给处理函数。
/// 使用类型安全的方式进行事件分发。
///
/// # 设计说明
///
/// ## 与 DistEngine 的对比
///
/// - **C++ 版本**：使用模板和 `dynamic_cast` 进行类型转换
/// - **Rust 版本**：使用枚举比较和 trait 对象，避免了不安全的类型转换
///
/// ## 生命周期说明
///
/// 分发器持有事件的可变引用（`&'a mut`），在分发器存在期间，
/// 事件不能被其他代码访问。这确保了线程安全和内存安全。
///
/// # 使用模式
///
/// ## 基本模式
///
/// ```
/// use DistRender::core::event::{Event, EventType, EventDispatcher, WindowResizeEvent};
///
/// let mut event = WindowResizeEvent::new(1920, 1080);
/// let mut dispatcher = EventDispatcher::new(&mut event);
///
/// // 分发事件
/// let handled = dispatcher.dispatch(EventType::WindowResize, |e| {
///     println!("处理窗口调整: {}", e.detail());
///     true
/// });
/// ```
///
/// ## 责任链模式
///
/// ```
/// use DistRender::core::event::{Event, EventType, EventDispatcher, KeyboardEvent, KeyCode};
///
/// let mut event = KeyboardEvent::pressed(KeyCode::Escape);
/// let mut dispatcher = EventDispatcher::new(&mut event);
///
/// // 第一个处理器
/// dispatcher.dispatch(EventType::KeyDown, |e| {
///     println!("处理器 1");
///     false // 继续传递
/// });
///
/// // 第二个处理器
/// if !dispatcher.is_handled() {
///     dispatcher.dispatch(EventType::KeyDown, |e| {
///         println!("处理器 2");
///         true // 停止传递
///     });
/// }
/// ```
///
/// # 性能考虑
///
/// - 类型检查在编译时完成，无运行时开销
/// - 闭包内联优化，接近手写代码的性能
/// - 无需堆分配或虚函数调用
pub struct EventDispatcher<'a> {
    /// 事件引用
    ///
    /// 持有对事件的可变引用，允许修改事件状态（如 handled 标志）
    event: &'a mut dyn Event,
}

impl<'a> EventDispatcher<'a> {
    /// 创建新的事件分发器
    ///
    /// # 参数
    ///
    /// * `event` - 要分发的事件的可变引用
    ///
    /// # 返回值
    ///
    /// 返回新创建的事件分发器
    ///
    /// # 示例
    ///
    /// ```
    /// use DistRender::core::event::{EventDispatcher, WindowResizeEvent};
    ///
    /// let mut event = WindowResizeEvent::new(800, 600);
    /// let dispatcher = EventDispatcher::new(&mut event);
    /// ```
    pub fn new(event: &'a mut dyn Event) -> Self {
        Self { event }
    }

    /// 分发事件到处理函数
    ///
    /// 如果事件类型匹配，则调用处理函数。
    /// 处理函数返回 true 表示事件已处理，会自动设置事件的处理状态。
    ///
    /// # 类型检查
    ///
    /// 分发器会比较事件的实际类型与期望类型：
    /// - 匹配：调用处理函数
    /// - 不匹配：返回 false，不调用处理函数
    ///
    /// # 参数
    ///
    /// * `event_type` - 期望的事件类型
    /// * `handler` - 事件处理函数，接收 `&mut dyn Event`，返回 `bool`
    ///
    /// # 返回值
    ///
    /// - `true`: 事件类型匹配且处理函数返回 true
    /// - `false`: 事件类型不匹配或处理函数返回 false
    ///
    /// # 示例
    ///
    /// ```
    /// use DistRender::core::event::{Event, EventType, EventDispatcher, WindowResizeEvent};
    ///
    /// let mut event = WindowResizeEvent::new(1920, 1080);
    /// let mut dispatcher = EventDispatcher::new(&mut event);
    ///
    /// // 正确的类型 - 会被调用
    /// let result = dispatcher.dispatch(EventType::WindowResize, |e| {
    ///     println!("事件详情: {}", e.detail());
    ///     true
    /// });
    /// assert!(result);
    ///
    /// // 错误的类型 - 不会被调用
    /// let result = dispatcher.dispatch(EventType::MouseButtonDown, |_| {
    ///     panic!("不应该被调用");
    /// });
    /// assert!(!result);
    /// ```
    pub fn dispatch<F>(&mut self, event_type: EventType, mut handler: F) -> bool
    where
        F: FnMut(&mut dyn Event) -> bool,
    {
        // 类型检查：只有当事件类型匹配时才调用处理函数
        if self.event.event_type() == event_type {
            let handled = handler(self.event);
            self.event.set_handled(handled);
            handled
        } else {
            false
        }
    }

    /// 获取事件是否已被处理
    ///
    /// 用于实现事件处理链，检查事件是否需要继续传递。
    ///
    /// # 返回值
    ///
    /// - `true`: 事件已被某个处理器处理
    /// - `false`: 事件尚未被处理
    ///
    /// # 示例
    ///
    /// ```
    /// use DistRender::core::event::{Event, EventType, EventDispatcher, WindowResizeEvent};
    ///
    /// let mut event = WindowResizeEvent::new(800, 600);
    /// let mut dispatcher = EventDispatcher::new(&mut event);
    ///
    /// assert!(!dispatcher.is_handled());
    ///
    /// dispatcher.dispatch(EventType::WindowResize, |_| true);
    ///
    /// assert!(dispatcher.is_handled());
    /// ```
    pub fn is_handled(&self) -> bool {
        self.event.is_handled()
    }
}

/// 事件处理器 trait
///
/// 实现此 trait 的类型可以处理事件，通常用于游戏对象、场景、层等。
///
/// # 设计说明
///
/// 此 trait 提供了统一的事件处理接口，允许不同类型的对象
/// 以相同的方式处理事件。这在实现层系统（Layer System）时特别有用。
///
/// # 使用场景
///
/// - 游戏层（GameLayer、UILayer 等）
/// - 场景对象（Scene）
/// - 游戏实体（Entity）
/// - 应用程序（Application）
///
/// # 示例
///
/// ```
/// use DistRender::core::event::{Event, EventType, EventHandler, KeyboardEvent, KeyCode};
///
/// // 定义一个游戏层
/// struct GameLayer {
///     player_speed: f32,
/// }
///
/// impl EventHandler for GameLayer {
///     fn handle_event(&mut self, event: &mut dyn Event) -> bool {
///         match event.event_type() {
///             EventType::KeyDown => {
///                 // 处理键盘输入
///                 println!("游戏层处理按键事件");
///                 true
///             }
///             EventType::WindowResize => {
///                 // 处理窗口调整
///                 println!("游戏层处理窗口调整");
///                 true
///             }
///             _ => false, // 不处理其他事件
///         }
///     }
/// }
///
/// // 使用
/// let mut layer = GameLayer { player_speed: 5.0 };
/// let mut event = KeyboardEvent::pressed(KeyCode::W);
/// let handled = layer.handle_event(&mut event);
/// ```
///
/// # 层系统示例
///
/// ```
/// use DistRender::core::event::{Event, EventHandler};
///
/// struct LayerStack {
///     layers: Vec<Box<dyn EventHandler>>,
/// }
///
/// impl LayerStack {
///     fn handle_event(&mut self, event: &mut dyn Event) {
///         // 从顶层到底层传递事件
///         for layer in self.layers.iter_mut().rev() {
///             if layer.handle_event(event) {
///                 break; // 事件已被处理，停止传递
///             }
///         }
///     }
/// }
/// ```
pub trait EventHandler {
    /// 处理事件
    ///
    /// 处理给定的事件，并返回是否成功处理。
    /// 如果返回 `true`，事件通常不会继续传递给其他处理器。
    ///
    /// # 参数
    ///
    /// * `event` - 要处理的事件的可变引用
    ///
    /// # 返回值
    ///
    /// - `true`: 事件已被此处理器处理
    /// - `false`: 事件未被处理，应继续传递
    ///
    /// # 实现建议
    ///
    /// - 使用 `match event.event_type()` 判断事件类型
    /// - 只处理关心的事件类型，其他返回 `false`
    /// - 处理完成后根据情况决定返回 `true` 或 `false`
    /// - 如果希望事件继续传递，返回 `false`
    ///
    /// # 示例
    ///
    /// ```
    /// use DistRender::core::event::{Event, EventType, EventHandler};
    ///
    /// struct MyHandler;
    ///
    /// impl EventHandler for MyHandler {
    ///     fn handle_event(&mut self, event: &mut dyn Event) -> bool {
    ///         match event.event_type() {
    ///             EventType::WindowResize => {
    ///                 println!("处理窗口调整: {}", event.detail());
    ///                 true // 事件已处理
    ///             }
    ///             EventType::KeyDown => {
    ///                 println!("监控按键: {}", event.detail());
    ///                 false // 继续传递给其他处理器
    ///             }
    ///             _ => false, // 不处理其他事件
    ///         }
    ///     }
    /// }
    /// ```
    fn handle_event(&mut self, event: &mut dyn Event) -> bool;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_types() {
        assert_eq!(EventType::WindowResize.name(), "WindowResize");
        assert_eq!(EventType::MouseButtonDown.name(), "MouseButtonDown");
        assert_eq!(EventType::KeyDown.name(), "KeyDown");
    }

    #[test]
    fn test_window_resize_event() {
        let mut event = WindowResizeEvent::new(1920, 1080);
        assert_eq!(event.event_type(), EventType::WindowResize);
        assert_eq!(event.width, 1920);
        assert_eq!(event.height, 1080);
        assert!(!event.is_handled());

        event.set_handled(true);
        assert!(event.is_handled());
    }

    #[test]
    fn test_mouse_button_event() {
        let mut event = MouseButtonEvent::pressed(MouseButton::Left, 100.0, 200.0);
        assert_eq!(event.event_type(), EventType::MouseButtonDown);
        assert_eq!(event.button, MouseButton::Left);
        assert!(event.pressed);

        let mut event = MouseButtonEvent::released(MouseButton::Right, 150.0, 250.0);
        assert_eq!(event.event_type(), EventType::MouseButtonUp);
        assert!(!event.pressed);
    }

    #[test]
    fn test_keyboard_event() {
        let mut event = KeyboardEvent::pressed(KeyCode::W);
        assert_eq!(event.event_type(), EventType::KeyDown);
        assert_eq!(event.key_code, KeyCode::W);
        assert!(event.pressed);
    }

    #[test]
    fn test_event_dispatcher() {
        let mut event = WindowResizeEvent::new(800, 600);
        let mut dispatcher = EventDispatcher::new(&mut event);

        let handled = dispatcher.dispatch(EventType::WindowResize, |e| {
            // 检查事件类型
            assert_eq!(e.event_type(), EventType::WindowResize);
            true // 标记为已处理
        });

        assert!(handled);
        assert!(dispatcher.is_handled());
    }

    #[test]
    fn test_event_dispatcher_type_mismatch() {
        let mut event = WindowResizeEvent::new(800, 600);
        let mut dispatcher = EventDispatcher::new(&mut event);

        let handled = dispatcher.dispatch(EventType::MouseButtonDown, |_e| {
            // 这个处理器不应该被调用
            panic!("Should not be called");
        });

        assert!(!handled);
        assert!(!dispatcher.is_handled());
    }

    #[test]
    fn test_tick_event() {
        let event = TickEvent::new(0.016, 1.5);
        assert_eq!(event.delta_time, 0.016);
        assert_eq!(event.total_time, 1.5);
        assert_eq!(event.event_type(), EventType::Tick);
    }
}
