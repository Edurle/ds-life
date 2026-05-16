use std::collections::VecDeque;

/// 滑动窗口崩溃检测器。
///
/// 记录最近 N 次执行的成功/失败状态，
/// 当失败率超过阈值或连续失败次数超标时建议回滚。
pub struct CrashDetector {
    window: VecDeque<bool>,
    max_size: usize,
    consecutive_failures: usize,
}

impl CrashDetector {
    pub fn new(window_size: usize) -> Self {
        Self {
            window: VecDeque::with_capacity(window_size),
            max_size: window_size,
            consecutive_failures: 0,
        }
    }

    /// 记录一次执行结果。
    /// - `success = true`: 重置连续失败计数
    /// - `success = false`: 连续失败计数 +1
    pub fn record(&mut self, success: bool) {
        self.window.push_back(success);
        if self.window.len() > self.max_size {
            self.window.pop_front();
        }
        if !success {
            self.consecutive_failures += 1;
        } else {
            self.consecutive_failures = 0;
        }
    }

    /// 判断是否需要回滚。
    ///
    /// - `threshold`: 滑动窗口中的失败率阈值（如 0.6 表示 60%）
    /// - `max_consecutive`: 最大允许连续失败次数
    ///
    /// 窗口未填满时返回 `false`。
    pub fn should_rollback(&self, threshold: f64, max_consecutive: usize) -> bool {
        let total = self.window.len() as f64;
        if total < self.max_size as f64 {
            return false;
        }
        let failures = self.window.iter().filter(|&&s| !s).count() as f64;
        let failure_rate = failures / total;
        failure_rate >= threshold || self.consecutive_failures >= max_consecutive
    }
}
