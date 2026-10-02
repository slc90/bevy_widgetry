use bevy::prelude::*;

/// 同一 channel 的两种 representation 共用颜色；lane 等高且没有 gap/separator。
/// 可修改的展示配置 Component，不是 runtime 数据结果；修改后由 renderer 校验和同步。
/// 它与 Cursor / OutputLength 的外部输入职责独立，不提供数据提交或显示完成通知。
#[derive(Component, Clone)]
pub struct WaveformStyle {
    /// 按 channel index 循环；至少两个相邻不同的有限颜色。
    pub palette: Vec<Color>,
    /// physical pixels 为单位的线宽，必须有限且严格大于零。
    pub line_width: f32,
    /// offscreen target 的背景色，不增加 lane 分隔线。
    pub background: Color,
}

impl Default for WaveformStyle {
    fn default() -> Self {
        Self {
            palette: vec![
                Color::srgb(0.2, 0.8, 0.95),
                Color::srgb(1.0, 0.6, 0.3),
                Color::srgb(0.55, 0.9, 0.4),
                Color::srgb(0.85, 0.45, 0.95),
            ],
            line_width: 1.0,
            background: Color::srgb(0.03, 0.04, 0.06),
        }
    }
}

impl WaveformStyle {
    /// Scene 与 runtime style mutation 共享检查；拒绝非法 palette 或线宽。
    pub(crate) fn validate(&self) -> Result<(), BevyError> {
        let finite = |color: &Color| {
            color
                .to_linear()
                .to_f32_array()
                .iter()
                .all(|value| value.is_finite())
        };
        if self.palette.len() < 2
            || !self.line_width.is_finite()
            || self.line_width <= 0.0
            || !finite(&self.background)
            || self.palette.iter().any(|color| !finite(color))
            || self
                .palette
                .iter()
                .zip(self.palette.iter().cycle().skip(1))
                .any(|(a, b)| a.to_linear() == b.to_linear())
        {
            return Err(BevyError::error("Invalid WaveformStyle"));
        }
        Ok(())
    }
}
