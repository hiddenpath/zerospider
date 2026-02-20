//! Provider scoring system for intelligent model selection.
//!
//! Evaluates providers across multiple dimensions:
//! - Latency: Response time
//! - Cost: Token pricing
//! - Reliability: Success rate
//! - Quality: Output quality score

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Default)]
pub struct ProviderMetrics {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub total_latency_ms: u64,
    pub total_tokens: u64,
    pub total_cost_cents: u64,
    pub quality_scores: Vec<f64>,
    pub last_error: Option<String>,
    pub last_success: Option<Instant>,
}

impl ProviderMetrics {
    pub fn success_rate(&self) -> f64 {
        if self.total_requests == 0 {
            return 1.0;
        }
        self.successful_requests as f64 / self.total_requests as f64
    }

    pub fn avg_latency_ms(&self) -> f64 {
        if self.successful_requests == 0 {
            return 0.0;
        }
        self.total_latency_ms as f64 / self.successful_requests as f64
    }

    pub fn avg_quality(&self) -> f64 {
        if self.quality_scores.is_empty() {
            return 0.5;
        }
        self.quality_scores.iter().sum::<f64>() / self.quality_scores.len() as f64
    }

    pub fn cost_per_1k_tokens(&self) -> f64 {
        if self.total_tokens == 0 {
            return 0.0;
        }
        (self.total_cost_cents as f64 / self.total_tokens as f64) * 1000.0
    }
}

#[derive(Debug, Clone)]
pub struct ScoringWeights {
    pub latency: f64,
    pub cost: f64,
    pub reliability: f64,
    pub quality: f64,
}

impl Default for ScoringWeights {
    fn default() -> Self {
        Self {
            latency: 0.25,
            cost: 0.25,
            reliability: 0.30,
            quality: 0.20,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ScoringConfig {
    pub weights: ScoringWeights,
    pub latency_threshold_ms: u64,
    pub cost_threshold_cents: u64,
    pub min_requests_for_scoring: u64,
}

impl Default for ScoringConfig {
    fn default() -> Self {
        Self {
            weights: ScoringWeights::default(),
            latency_threshold_ms: 5000,
            cost_threshold_cents: 100,
            min_requests_for_scoring: 10,
        }
    }
}

pub struct ProviderScorer {
    metrics: HashMap<String, Arc<ProviderMetrics>>,
    config: ScoringConfig,
}

impl ProviderScorer {
    pub fn new(config: ScoringConfig) -> Self {
        Self {
            metrics: HashMap::new(),
            config,
        }
    }

    pub fn record_success(
        &mut self,
        provider_id: &str,
        latency: Duration,
        tokens: u64,
        cost_cents: u64,
        quality_score: Option<f64>,
    ) {
        let metrics = self
            .metrics
            .entry(provider_id.to_string())
            .or_insert_with(|| Arc::new(ProviderMetrics::default()));

        let m = Arc::get_mut(metrics).unwrap();
        m.total_requests += 1;
        m.successful_requests += 1;
        m.total_latency_ms += latency.as_millis() as u64;
        m.total_tokens += tokens;
        m.total_cost_cents += cost_cents;
        if let Some(score) = quality_score {
            m.quality_scores.push(score);
            if m.quality_scores.len() > 100 {
                m.quality_scores.remove(0);
            }
        }
        m.last_success = Some(Instant::now());
    }

    pub fn record_failure(&mut self, provider_id: &str, error: &str) {
        let metrics = self
            .metrics
            .entry(provider_id.to_string())
            .or_insert_with(|| Arc::new(ProviderMetrics::default()));

        let m = Arc::get_mut(metrics).unwrap();
        m.total_requests += 1;
        m.last_error = Some(error.to_string());
    }

    pub fn score(&self, provider_id: &str) -> f64 {
        let metrics = match self.metrics.get(provider_id) {
            Some(m) => m,
            None => return 0.5,
        };

        if metrics.total_requests < self.config.min_requests_for_scoring {
            return 0.5;
        }

        let latency_score = if metrics.avg_latency_ms() <= 0.0 {
            1.0
        } else {
            (self.config.latency_threshold_ms as f64 / metrics.avg_latency_ms()).min(1.0)
        };

        let cost_score = if metrics.cost_per_1k_tokens() <= 0.0 {
            1.0
        } else {
            (self.config.cost_threshold_cents as f64 / metrics.cost_per_1k_tokens()).min(1.0)
        };

        let reliability_score = metrics.success_rate();
        let quality_score = metrics.avg_quality();

        self.config.weights.latency * latency_score
            + self.config.weights.cost * cost_score
            + self.config.weights.reliability * reliability_score
            + self.config.weights.quality * quality_score
    }

    pub fn rank_providers(&self, provider_ids: &[&str]) -> Vec<(String, f64)> {
        let mut ranked: Vec<_> = provider_ids
            .iter()
            .map(|id| ((*id).to_string(), self.score(id)))
            .collect();
        ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        ranked
    }

    pub fn get_metrics(&self, provider_id: &str) -> Option<&ProviderMetrics> {
        self.metrics.get(provider_id).map(|arc| arc.as_ref())
    }
}

pub struct AtomicProviderMetrics {
    total_requests: AtomicU64,
    successful_requests: AtomicU64,
    total_latency_ms: AtomicU64,
    total_tokens: AtomicU64,
    total_cost_cents: AtomicU64,
}

impl Default for AtomicProviderMetrics {
    fn default() -> Self {
        Self {
            total_requests: AtomicU64::new(0),
            successful_requests: AtomicU64::new(0),
            total_latency_ms: AtomicU64::new(0),
            total_tokens: AtomicU64::new(0),
            total_cost_cents: AtomicU64::new(0),
        }
    }
}

impl AtomicProviderMetrics {
    pub fn snapshot(&self) -> ProviderMetrics {
        ProviderMetrics {
            total_requests: self.total_requests.load(Ordering::Relaxed),
            successful_requests: self.successful_requests.load(Ordering::Relaxed),
            total_latency_ms: self.total_latency_ms.load(Ordering::Relaxed),
            total_tokens: self.total_tokens.load(Ordering::Relaxed),
            total_cost_cents: self.total_cost_cents.load(Ordering::Relaxed),
            ..Default::default()
        }
    }

    pub fn record_success(&self, latency_ms: u64, tokens: u64, cost_cents: u64) {
        self.total_requests.fetch_add(1, Ordering::Relaxed);
        self.successful_requests.fetch_add(1, Ordering::Relaxed);
        self.total_latency_ms
            .fetch_add(latency_ms, Ordering::Relaxed);
        self.total_tokens.fetch_add(tokens, Ordering::Relaxed);
        self.total_cost_cents
            .fetch_add(cost_cents, Ordering::Relaxed);
    }

    pub fn record_failure(&self) {
        self.total_requests.fetch_add(1, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_provider_metrics_calculations() {
        let mut metrics = ProviderMetrics::default();
        metrics.total_requests = 100;
        metrics.successful_requests = 95;
        metrics.total_latency_ms = 5000;
        metrics.total_tokens = 10000;
        metrics.total_cost_cents = 50;
        metrics.quality_scores = vec![0.8, 0.9, 0.85];

        assert!((metrics.success_rate() - 0.95).abs() < 0.01);
        assert!((metrics.avg_latency_ms() - 52.63).abs() < 0.1);
        assert!((metrics.avg_quality() - 0.85).abs() < 0.01);
    }

    #[test]
    fn test_provider_scorer() {
        let mut scorer = ProviderScorer::new(ScoringConfig::default());

        scorer.record_success("openai", Duration::from_millis(100), 1000, 2, Some(0.9));
        scorer.record_success("openai", Duration::from_millis(150), 1000, 2, Some(0.85));
        scorer.record_success("anthropic", Duration::from_millis(200), 1000, 3, Some(0.95));
        scorer.record_failure("anthropic", "timeout");

        let openai_score = scorer.score("openai");
        let anthropic_score = scorer.score("anthropic");

        assert!(openai_score > 0.0);
        assert!(anthropic_score > 0.0);
    }

    #[test]
    fn test_rank_providers() {
        let mut scorer = ProviderScorer::new(ScoringConfig {
            min_requests_for_scoring: 1,
            ..Default::default()
        });

        for _ in 0..10 {
            scorer.record_success("fast-cheap", Duration::from_millis(50), 1000, 1, Some(0.8));
            scorer.record_success(
                "slow-expensive",
                Duration::from_millis(2000),
                1000,
                10,
                Some(0.9),
            );
        }

        let ranked = scorer.rank_providers(&["fast-cheap", "slow-expensive"]);
        assert_eq!(ranked[0].0, "fast-cheap");
    }
}
