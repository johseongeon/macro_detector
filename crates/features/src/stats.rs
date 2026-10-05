/// Welford 알고리즘 기반 증분 평균·분산. 이벤트당 O(1), 할당 없음.
#[derive(Clone, Copy, Debug, Default)]
pub struct RunningStats {
    n: u64,
    mean: f64,
    m2: f64,
}

impl RunningStats {
    #[inline]
    pub fn push(&mut self, x: f64) {
        self.n += 1;
        let delta = x - self.mean;
        self.mean += delta / self.n as f64;
        self.m2 += delta * (x - self.mean);
    }

    pub fn count(&self) -> u64 {
        self.n
    }

    pub fn mean(&self) -> f64 {
        self.mean
    }

    /// 표본 분산. 표본이 2개 미만이면 0.
    pub fn variance(&self) -> f64 {
        if self.n < 2 {
            0.0
        } else {
            self.m2 / (self.n - 1) as f64
        }
    }

    pub fn std(&self) -> f64 {
        self.variance().sqrt()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_two_pass_computation() {
        let xs = [2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0];
        let mut s = RunningStats::default();
        xs.iter().for_each(|&x| s.push(x));
        assert_eq!(s.count(), 8);
        assert!((s.mean() - 5.0).abs() < 1e-12);
        assert!((s.variance() - 32.0 / 7.0).abs() < 1e-12);
    }
}
