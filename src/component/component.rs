//! 组件基类
//!
//! 参考 DistEngine 的 Component.h 实现

/// 组件 trait
///
/// 所有游戏对象组件的基础接口
pub trait Component {
    /// 获取组件名称
    fn name(&self) -> &str;

    /// 每帧更新（可选实现）
    fn tick(&mut self, _delta_time: f32) {}
}
