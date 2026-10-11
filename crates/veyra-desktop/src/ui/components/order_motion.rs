//! 原版列表 FLIP 的共享180ms动画；反向时从当前画面位置连续开始。
use std::time::{Duration, Instant};

// React FLIP每次换位都从当前画面重新运行180ms；不能采用Kit快速反向缩短时长。
pub(crate) struct OrderMotion {
    pub(crate) from: f32,
    pub(crate) target: f32,
    pub(crate) started_at: Instant,
}
impl OrderMotion {
    pub(crate) fn sample(&self, now: Instant) -> f32 {
        let progress = (now.saturating_duration_since(self.started_at).as_secs_f32()
            / Duration::from_millis(super::super::tokens::REORDER_MOVE_MS).as_secs_f32())
        .min(1.);
        let eased = gpui_kit::component::animation::cubic_bezier(0.2, 0.8, 0.2, 1.)(progress);
        self.from + (self.target - self.from) * eased
    }
    pub(crate) fn retarget(&mut self, target: f32, now: Instant) {
        if self.target != target {
            self.from = self.sample(now);
            self.target = target;
            self.started_at = now;
        }
    }
}
