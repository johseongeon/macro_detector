//! 확률 보정. 학습 단계에서 구한 isotonic 회귀 결과를 구간 선형 함수로 적용한다.

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Calibrator {
    /// 오름차순 입력 확률.
    xs: Vec<f32>,
    /// 각 입력에 대응하는 보정 확률(단조 비감소).
    ys: Vec<f32>,
}

impl Calibrator {
    pub fn identity() -> Self {
        Self {
            xs: vec![0.0, 1.0],
            ys: vec![0.0, 1.0],
        }
    }

    pub fn new(xs: Vec<f32>, ys: Vec<f32>) -> Option<Self> {
        let valid = xs.len() >= 2
            && xs.len() == ys.len()
            && xs.windows(2).all(|w| w[0] < w[1])
            && ys.windows(2).all(|w| w[0] <= w[1]);
        valid.then_some(Self { xs, ys })
    }

    pub fn apply(&self, p: f32) -> f32 {
        let (xs, ys) = (&self.xs, &self.ys);
        if p <= xs[0] {
            return ys[0];
        }
        if p >= xs[xs.len() - 1] {
            return ys[ys.len() - 1];
        }
        // xs[i-1] < p <= xs[i]
        let i = xs.partition_point(|&x| x < p);
        let t = (p - xs[i - 1]) / (xs[i] - xs[i - 1]);
        ys[i - 1] + t * (ys[i] - ys[i - 1])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interpolates_and_clamps() {
        let c = Calibrator::new(vec![0.2, 0.6], vec![0.1, 0.9]).unwrap();
        assert_eq!(c.apply(0.0), 0.1);
        assert_eq!(c.apply(1.0), 0.9);
        assert!((c.apply(0.4) - 0.5).abs() < 1e-6);
    }

    #[test]
    fn rejects_non_monotonic() {
        assert!(Calibrator::new(vec![0.0, 1.0], vec![0.9, 0.1]).is_none());
    }
}
