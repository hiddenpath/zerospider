//! Adaptive model selector with task-aware routing.
//!
//! Selects the optimal model based on:
//! - Task type (coding, reasoning, creative, etc.)
//! - Provider scores and availability
//! - Cost constraints
//! - Latency requirements

use std::collections::HashMap;

use super::scoring::{ProviderScorer, ScoringWeights};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TaskType {
    Coding,
    Reasoning,
    Creative,
    Summarization,
    Translation,
    Chat,
    ToolUse,
    Vision,
    Embedding,
}

impl TaskType {
    pub fn from_prompt(prompt: &str) -> Self {
        let lower = prompt.to_lowercase();

        if lower.contains("code") || lower.contains("function") || lower.contains("debug") {
            return TaskType::Coding;
        }
        if lower.contains("analyze") || lower.contains("reason") || lower.contains("think") {
            return TaskType::Reasoning;
        }
        if lower.contains("write") || lower.contains("story") || lower.contains("creative") {
            return TaskType::Creative;
        }
        if lower.contains("summarize") || lower.contains("summarize") || lower.contains("brief") {
            return TaskType::Summarization;
        }
        if lower.contains("translate") || lower.contains("翻译") {
            return TaskType::Translation;
        }
        if lower.contains("tool") || lower.contains("execute") || lower.contains("call") {
            return TaskType::ToolUse;
        }
        if lower.contains("image") || lower.contains("picture") || lower.contains("screenshot") {
            return TaskType::Vision;
        }

        TaskType::Chat
    }
}

#[derive(Debug, Clone)]
pub struct ModelProfile {
    pub provider_id: String,
    pub model_id: String,
    pub supported_tasks: Vec<TaskType>,
    pub context_window: u32,
    pub supports_vision: bool,
    pub supports_tools: bool,
    pub supports_streaming: bool,
    pub input_cost_per_1k: f64,
    pub output_cost_per_1k: f64,
    pub avg_latency_ms: u64,
}

impl ModelProfile {
    pub fn matches_task(&self, task: TaskType) -> bool {
        self.supported_tasks.contains(&task) || self.supported_tasks.is_empty()
    }
}

#[derive(Debug, Clone)]
pub struct SelectionCriteria {
    pub task_type: Option<TaskType>,
    pub max_latency_ms: Option<u64>,
    pub max_cost_per_1k: Option<f64>,
    pub min_context_window: Option<u32>,
    pub require_vision: bool,
    pub require_tools: bool,
    pub prefer_quality: bool,
}

impl Default for SelectionCriteria {
    fn default() -> Self {
        Self {
            task_type: None,
            max_latency_ms: None,
            max_cost_per_1k: None,
            min_context_window: None,
            require_vision: false,
            require_tools: false,
            prefer_quality: false,
        }
    }
}

pub struct AdaptiveSelector {
    models: HashMap<String, ModelProfile>,
    scorer: ProviderScorer,
    task_weights: HashMap<TaskType, ScoringWeights>,
}

impl AdaptiveSelector {
    pub fn new(scorer: ProviderScorer) -> Self {
        let mut task_weights = HashMap::new();

        task_weights.insert(
            TaskType::Coding,
            ScoringWeights {
                quality: 0.40,
                reliability: 0.30,
                latency: 0.20,
                cost: 0.10,
            },
        );

        task_weights.insert(
            TaskType::Reasoning,
            ScoringWeights {
                quality: 0.50,
                reliability: 0.30,
                latency: 0.15,
                cost: 0.05,
            },
        );

        task_weights.insert(
            TaskType::Chat,
            ScoringWeights {
                latency: 0.40,
                cost: 0.30,
                reliability: 0.20,
                quality: 0.10,
            },
        );

        task_weights.insert(
            TaskType::ToolUse,
            ScoringWeights {
                reliability: 0.40,
                quality: 0.30,
                latency: 0.20,
                cost: 0.10,
            },
        );

        Self {
            models: HashMap::new(),
            scorer,
            task_weights,
        }
    }

    pub fn register_model(&mut self, profile: ModelProfile) {
        let key = format!("{}/{}", profile.provider_id, profile.model_id);
        self.models.insert(key, profile);
    }

    pub fn select_best(&self, criteria: &SelectionCriteria) -> Option<&ModelProfile> {
        let candidates: Vec<_> = self
            .models
            .values()
            .filter(|m| self.matches_criteria(m, criteria))
            .collect();

        if candidates.is_empty() {
            return None;
        }

        candidates
            .iter()
            .max_by(|a, b| {
                let score_a = self.compute_score(a, criteria);
                let score_b = self.compute_score(b, criteria);
                score_a
                    .partial_cmp(&score_b)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .copied()
    }

    pub fn select_top_n(&self, criteria: &SelectionCriteria, n: usize) -> Vec<&ModelProfile> {
        let mut candidates: Vec<_> = self
            .models
            .values()
            .filter(|m| self.matches_criteria(m, criteria))
            .collect();

        candidates.sort_by(|a, b| {
            let score_a = self.compute_score(a, criteria);
            let score_b = self.compute_score(b, criteria);
            score_b
                .partial_cmp(&score_a)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        candidates.into_iter().take(n).collect()
    }

    fn matches_criteria(&self, model: &ModelProfile, criteria: &SelectionCriteria) -> bool {
        if let Some(task) = criteria.task_type {
            if !model.matches_task(task) {
                return false;
            }
        }

        if let Some(max_latency) = criteria.max_latency_ms {
            if model.avg_latency_ms > max_latency {
                return false;
            }
        }

        if let Some(max_cost) = criteria.max_cost_per_1k {
            let total_cost = model.input_cost_per_1k + model.output_cost_per_1k;
            if total_cost > max_cost {
                return false;
            }
        }

        if let Some(min_ctx) = criteria.min_context_window {
            if model.context_window < min_ctx {
                return false;
            }
        }

        if criteria.require_vision && !model.supports_vision {
            return false;
        }

        if criteria.require_tools && !model.supports_tools {
            return false;
        }

        true
    }

    fn compute_score(&self, model: &ModelProfile, criteria: &SelectionCriteria) -> f64 {
        let weights = match criteria.task_type {
            Some(task) => self.task_weights.get(&task).cloned().unwrap_or_default(),
            None => ScoringWeights::default(),
        };

        let provider_score = self.scorer.score(&model.provider_id);

        let latency_score = if let Some(max) = criteria.max_latency_ms {
            (max as f64 / model.avg_latency_ms as f64).min(1.0)
        } else {
            1.0
        };

        let cost_score = {
            let total = model.input_cost_per_1k + model.output_cost_per_1k;
            if total <= 0.0 {
                1.0
            } else if let Some(max) = criteria.max_cost_per_1k {
                (max / total).min(1.0)
            } else {
                1.0
            }
        };

        weights.quality * provider_score
            + weights.reliability * provider_score
            + weights.latency * latency_score
            + weights.cost * cost_score
    }
}

pub fn default_model_profiles() -> Vec<ModelProfile> {
    vec![
        ModelProfile {
            provider_id: "openai".to_string(),
            model_id: "gpt-4o".to_string(),
            supported_tasks: vec![
                TaskType::Coding,
                TaskType::Reasoning,
                TaskType::Creative,
                TaskType::ToolUse,
                TaskType::Vision,
            ],
            context_window: 128000,
            supports_vision: true,
            supports_tools: true,
            supports_streaming: true,
            input_cost_per_1k: 0.005,
            output_cost_per_1k: 0.015,
            avg_latency_ms: 800,
        },
        ModelProfile {
            provider_id: "openai".to_string(),
            model_id: "gpt-4o-mini".to_string(),
            supported_tasks: vec![
                TaskType::Chat,
                TaskType::Summarization,
                TaskType::Translation,
            ],
            context_window: 128000,
            supports_vision: true,
            supports_tools: true,
            supports_streaming: true,
            input_cost_per_1k: 0.00015,
            output_cost_per_1k: 0.0006,
            avg_latency_ms: 300,
        },
        ModelProfile {
            provider_id: "anthropic".to_string(),
            model_id: "claude-sonnet-4-20250514".to_string(),
            supported_tasks: vec![
                TaskType::Coding,
                TaskType::Reasoning,
                TaskType::Creative,
                TaskType::ToolUse,
                TaskType::Vision,
            ],
            context_window: 200000,
            supports_vision: true,
            supports_tools: true,
            supports_streaming: true,
            input_cost_per_1k: 0.003,
            output_cost_per_1k: 0.015,
            avg_latency_ms: 600,
        },
        ModelProfile {
            provider_id: "deepseek".to_string(),
            model_id: "deepseek-chat".to_string(),
            supported_tasks: vec![TaskType::Coding, TaskType::Reasoning, TaskType::Chat],
            context_window: 64000,
            supports_vision: false,
            supports_tools: true,
            supports_streaming: true,
            input_cost_per_1k: 0.00014,
            output_cost_per_1k: 0.00028,
            avg_latency_ms: 500,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_type_detection() {
        assert_eq!(
            TaskType::from_prompt("Write a function to sort an array"),
            TaskType::Coding
        );
        assert_eq!(
            TaskType::from_prompt("Analyze the pros and cons"),
            TaskType::Reasoning
        );
        assert_eq!(
            TaskType::from_prompt("Translate this to Chinese"),
            TaskType::Translation
        );
    }

    #[test]
    fn test_adaptive_selector() {
        let scorer = ProviderScorer::new(ScoringConfig::default());
        let mut selector = AdaptiveSelector::new(scorer);

        for profile in default_model_profiles() {
            selector.register_model(profile);
        }

        let criteria = SelectionCriteria {
            task_type: Some(TaskType::Coding),
            require_tools: true,
            ..Default::default()
        };

        let best = selector.select_best(&criteria);
        assert!(best.is_some());
    }

    #[test]
    fn test_select_top_n() {
        let scorer = ProviderScorer::new(ScoringConfig::default());
        let mut selector = AdaptiveSelector::new(scorer);

        for profile in default_model_profiles() {
            selector.register_model(profile);
        }

        let criteria = SelectionCriteria {
            task_type: Some(TaskType::Chat),
            ..Default::default()
        };

        let top = selector.select_top_n(&criteria, 2);
        assert!(top.len() <= 2);
    }
}
