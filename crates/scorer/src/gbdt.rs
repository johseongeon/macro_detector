//! Stage 2: 평탄화된 GBDT 추론기.
//!
//! 노드를 구조체 배열이 아닌 필드별 배열로 저장해 캐시 효율을 높인다.
//! 모델 파일은 `ml/guard_ml/export/flat_forest.py`가 생성한다.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// `feature`가 이 값이면 리프 노드.
pub const LEAF: u16 = u16::MAX;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FlatForest {
    pub base_score: f32,
    pub feature_count: usize,
    /// 각 트리의 루트 노드 인덱스.
    pub roots: Vec<u32>,
    pub feature: Vec<u16>,
    /// `x[feature] <= threshold`이면 왼쪽.
    pub threshold: Vec<f32>,
    pub left: Vec<u32>,
    pub right: Vec<u32>,
    /// 리프의 출력값(margin).
    pub value: Vec<f32>,
}

#[derive(Debug, Error, PartialEq)]
pub enum ForestError {
    #[error("node arrays have mismatched lengths")]
    LengthMismatch,
    #[error("node {0} references feature {1} outside the feature vector")]
    FeatureOutOfRange(usize, u16),
    #[error("node {0} has a child that does not come after it")]
    BadChild(usize),
    #[error("root {0} is out of range")]
    BadRoot(u32),
}

impl FlatForest {
    /// 트리 없이 상수 margin만 내는 모델.
    pub fn constant(base_score: f32) -> Self {
        Self {
            base_score,
            feature_count: 0,
            roots: vec![],
            feature: vec![],
            threshold: vec![],
            left: vec![],
            right: vec![],
            value: vec![],
        }
    }

    pub fn from_json(s: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let f: Self = serde_json::from_str(s)?;
        f.validate()?;
        Ok(f)
    }

    /// 로드 시 한 번 검증해, 추론 루프에서 경계 검사 실패나 무한 루프가 없도록 보장한다.
    /// 자식 인덱스가 항상 부모보다 커야 하므로 순회는 반드시 종료된다.
    pub fn validate(&self) -> Result<(), ForestError> {
        let n = self.feature.len();
        if [
            self.threshold.len(),
            self.left.len(),
            self.right.len(),
            self.value.len(),
        ]
        .iter()
        .any(|&l| l != n)
        {
            return Err(ForestError::LengthMismatch);
        }
        for i in 0..n {
            let f = self.feature[i];
            if f == LEAF {
                continue;
            }
            if f as usize >= self.feature_count {
                return Err(ForestError::FeatureOutOfRange(i, f));
            }
            for child in [self.left[i], self.right[i]] {
                if child as usize <= i || child as usize >= n {
                    return Err(ForestError::BadChild(i));
                }
            }
        }
        if let Some(&r) = self.roots.iter().find(|&&r| r as usize >= n) {
            return Err(ForestError::BadRoot(r));
        }
        Ok(())
    }

    /// `x.len()`는 `feature_count` 이상이어야 한다.
    #[inline]
    pub fn predict_margin(&self, x: &[f32]) -> f32 {
        debug_assert!(x.len() >= self.feature_count);
        let mut sum = self.base_score;
        for &root in &self.roots {
            let mut i = root as usize;
            loop {
                let f = self.feature[i];
                if f == LEAF {
                    sum += self.value[i];
                    break;
                }
                i = if x[f as usize] <= self.threshold[i] {
                    self.left[i] as usize
                } else {
                    self.right[i] as usize
                };
            }
        }
        sum
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 트리 1개: x0 <= 0.5 ? -1.0 : 2.0
    fn stump() -> FlatForest {
        FlatForest {
            base_score: 0.25,
            feature_count: 1,
            roots: vec![0],
            feature: vec![0, LEAF, LEAF],
            threshold: vec![0.5, 0.0, 0.0],
            left: vec![1, 0, 0],
            right: vec![2, 0, 0],
            value: vec![0.0, -1.0, 2.0],
        }
    }

    #[test]
    fn predicts_stump() {
        let f = stump();
        f.validate().unwrap();
        assert_eq!(f.predict_margin(&[0.1]), -0.75);
        assert_eq!(f.predict_margin(&[0.9]), 2.25);
    }

    #[test]
    fn rejects_backward_child() {
        let mut f = stump();
        f.left[0] = 0;
        assert_eq!(f.validate(), Err(ForestError::BadChild(0)));
    }

    #[test]
    fn rejects_out_of_range_feature() {
        let mut f = stump();
        f.feature[0] = 3;
        assert_eq!(f.validate(), Err(ForestError::FeatureOutOfRange(0, 3)));
    }

    #[test]
    fn roundtrips_json() {
        let s = serde_json::to_string(&stump()).unwrap();
        let f = FlatForest::from_json(&s).unwrap();
        assert_eq!(f.predict_margin(&[0.9]), 2.25);
    }
}
