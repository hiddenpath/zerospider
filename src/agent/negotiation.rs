//! Multi-model negotiation and decision making.
//!
//! Implements various negotiation strategies:
//! - Voting: Majority wins
//! - Consensus: All models agree
//! - Arbitration: One model decides
//! - Cascade: Sequential refinement

use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NegotiationStrategy {
    Voting,
    Consensus,
    Arbitration,
    Cascade,
    BestOfN,
    SelfConsistency,
}

#[derive(Debug, Clone)]
pub struct ModelResponse {
    pub model_id: String,
    pub content: String,
    pub confidence: Option<f64>,
    pub reasoning: Option<String>,
}

#[derive(Debug, Clone)]
pub struct NegotiationResult {
    pub final_response: String,
    pub strategy_used: NegotiationStrategy,
    pub participating_models: Vec<String>,
    pub agreement_score: f64,
    pub reasoning: Option<String>,
}

pub struct Negotiator {
    strategy: NegotiationStrategy,
    agreement_threshold: f64,
    max_rounds: u32,
}

impl Negotiator {
    pub fn new(strategy: NegotiationStrategy) -> Self {
        Self {
            strategy,
            agreement_threshold: 0.7,
            max_rounds: 3,
        }
    }

    pub fn with_agreement_threshold(mut self, threshold: f64) -> Self {
        self.agreement_threshold = threshold.clamp(0.0, 1.0);
        self
    }

    pub fn with_max_rounds(mut self, rounds: u32) -> Self {
        self.max_rounds = rounds;
        self
    }

    pub fn negotiate(&self, responses: Vec<ModelResponse>) -> NegotiationResult {
        if responses.is_empty() {
            return NegotiationResult {
                final_response: String::new(),
                strategy_used: self.strategy,
                participating_models: vec![],
                agreement_score: 0.0,
                reasoning: Some("No responses to negotiate".to_string()),
            };
        }

        if responses.len() == 1 {
            return NegotiationResult {
                final_response: responses[0].content.clone(),
                strategy_used: self.strategy,
                participating_models: vec![responses[0].model_id.clone()],
                agreement_score: 1.0,
                reasoning: None,
            };
        }

        match self.strategy {
            NegotiationStrategy::Voting => self.voting(responses),
            NegotiationStrategy::Consensus => self.consensus(responses),
            NegotiationStrategy::Arbitration => self.arbitration(responses),
            NegotiationStrategy::Cascade => self.cascade(responses),
            NegotiationStrategy::BestOfN => self.best_of_n(responses),
            NegotiationStrategy::SelfConsistency => self.self_consistency(responses),
        }
    }

    fn voting(&self, responses: Vec<ModelResponse>) -> NegotiationResult {
        let mut vote_counts: HashMap<String, usize> = HashMap::new();

        for response in &responses {
            let key = self.normalize_response(&response.content);
            *vote_counts.entry(key).or_insert(0) += 1;
        }

        let (winner, votes) = vote_counts
            .into_iter()
            .max_by_key(|(_, count)| *count)
            .unwrap_or((String::new(), 0));

        let agreement_score = votes as f64 / responses.len() as f64;

        NegotiationResult {
            final_response: winner,
            strategy_used: NegotiationStrategy::Voting,
            participating_models: responses.iter().map(|r| r.model_id.clone()).collect(),
            agreement_score,
            reasoning: Some(format!("{} of {} models agreed", votes, responses.len())),
        }
    }

    fn consensus(&self, responses: Vec<ModelResponse>) -> NegotiationResult {
        let normalized: Vec<_> = responses
            .iter()
            .map(|r| self.normalize_response(&r.content))
            .collect();

        if normalized.iter().all(|r| r == &normalized[0]) {
            return NegotiationResult {
                final_response: normalized[0].clone(),
                strategy_used: NegotiationStrategy::Consensus,
                participating_models: responses.iter().map(|r| r.model_id.clone()).collect(),
                agreement_score: 1.0,
                reasoning: Some("All models reached consensus".to_string()),
            };
        }

        NegotiationResult {
            final_response: responses[0].content.clone(),
            strategy_used: NegotiationStrategy::Consensus,
            participating_models: responses.iter().map(|r| r.model_id.clone()).collect(),
            agreement_score: 0.0,
            reasoning: Some("Consensus not reached, using first response".to_string()),
        }
    }

    fn arbitration(&self, responses: Vec<ModelResponse>) -> NegotiationResult {
        let arbitrator = responses
            .iter()
            .max_by(|a, b| {
                let conf_a = a.confidence.unwrap_or(0.5);
                let conf_b = b.confidence.unwrap_or(0.5);
                conf_a
                    .partial_cmp(&conf_b)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .unwrap_or(&responses[0]);

        NegotiationResult {
            final_response: arbitrator.content.clone(),
            strategy_used: NegotiationStrategy::Arbitration,
            participating_models: responses.iter().map(|r| r.model_id.clone()).collect(),
            agreement_score: arbitrator.confidence.unwrap_or(0.5),
            reasoning: Some(format!(
                "Arbitrator: {} (confidence: {:?})",
                arbitrator.model_id, arbitrator.confidence
            )),
        }
    }

    fn cascade(&self, responses: Vec<ModelResponse>) -> NegotiationResult {
        let mut current = responses[0].content.clone();
        let mut refinement_history = vec![responses[0].model_id.clone()];

        for response in responses.iter().skip(1) {
            if let Some(reasoning) = &response.reasoning {
                if reasoning.to_lowercase().contains("refine")
                    || reasoning.to_lowercase().contains("improve")
                {
                    current = response.content.clone();
                    refinement_history.push(response.model_id.clone());
                }
            }
        }

        NegotiationResult {
            final_response: current,
            strategy_used: NegotiationStrategy::Cascade,
            participating_models: responses.iter().map(|r| r.model_id.clone()).collect(),
            agreement_score: 1.0 - (1.0 / (refinement_history.len() as f64 + 1.0)),
            reasoning: Some(format!(
                "Refined through {} models",
                refinement_history.len()
            )),
        }
    }

    fn best_of_n(&self, responses: Vec<ModelResponse>) -> NegotiationResult {
        let best = responses
            .iter()
            .max_by(|a, b| {
                let score_a = a.confidence.unwrap_or(0.5);
                let score_b = b.confidence.unwrap_or(0.5);
                score_a
                    .partial_cmp(&score_b)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .unwrap_or(&responses[0]);

        NegotiationResult {
            final_response: best.content.clone(),
            strategy_used: NegotiationStrategy::BestOfN,
            participating_models: responses.iter().map(|r| r.model_id.clone()).collect(),
            agreement_score: best.confidence.unwrap_or(0.5),
            reasoning: Some(format!("Selected best from {} responses", responses.len())),
        }
    }

    fn self_consistency(&self, responses: Vec<ModelResponse>) -> NegotiationResult {
        let mut answer_clusters: HashMap<String, (String, usize)> = HashMap::new();

        for response in &responses {
            let normalized = self.normalize_response(&response.content);
            let entry = answer_clusters
                .entry(normalized.clone())
                .or_insert_with(|| (response.content.clone(), 0));
            entry.1 += 1;
        }

        let (_normalized_answer, (final_response, count)) = answer_clusters
            .into_iter()
            .max_by_key(|(_, (_, count))| *count)
            .unwrap_or((String::new(), (String::new(), 0)));

        let agreement_score = count as f64 / responses.len() as f64;

        NegotiationResult {
            final_response,
            strategy_used: NegotiationStrategy::SelfConsistency,
            participating_models: responses.iter().map(|r| r.model_id.clone()).collect(),
            agreement_score,
            reasoning: Some(format!("Self-consistency: {} identical responses", count)),
        }
    }

    fn normalize_response(&self, content: &str) -> String {
        content
            .to_lowercase()
            .chars()
            .filter(|c| c.is_alphanumeric() || c.is_whitespace())
            .collect::<String>()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    }
}

impl Default for Negotiator {
    fn default() -> Self {
        Self::new(NegotiationStrategy::Voting)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_voting_strategy() {
        let negotiator = Negotiator::new(NegotiationStrategy::Voting);

        let responses = vec![
            ModelResponse {
                model_id: "model-a".to_string(),
                content: "The answer is 42".to_string(),
                confidence: None,
                reasoning: None,
            },
            ModelResponse {
                model_id: "model-b".to_string(),
                content: "The answer is 42".to_string(),
                confidence: None,
                reasoning: None,
            },
            ModelResponse {
                model_id: "model-c".to_string(),
                content: "The answer is 24".to_string(),
                confidence: None,
                reasoning: None,
            },
        ];

        let result = negotiator.negotiate(responses);
        assert!(result.agreement_score > 0.5);
        assert!(result.final_response.contains("42"));
    }

    #[test]
    fn test_consensus_strategy() {
        let negotiator = Negotiator::new(NegotiationStrategy::Consensus);

        let responses = vec![
            ModelResponse {
                model_id: "model-a".to_string(),
                content: "Same answer".to_string(),
                confidence: None,
                reasoning: None,
            },
            ModelResponse {
                model_id: "model-b".to_string(),
                content: "Same answer".to_string(),
                confidence: None,
                reasoning: None,
            },
        ];

        let result = negotiator.negotiate(responses);
        assert_eq!(result.agreement_score, 1.0);
    }

    #[test]
    fn test_arbitration_strategy() {
        let negotiator = Negotiator::new(NegotiationStrategy::Arbitration);

        let responses = vec![
            ModelResponse {
                model_id: "low-confidence".to_string(),
                content: "Answer A".to_string(),
                confidence: Some(0.3),
                reasoning: None,
            },
            ModelResponse {
                model_id: "high-confidence".to_string(),
                content: "Answer B".to_string(),
                confidence: Some(0.95),
                reasoning: None,
            },
        ];

        let result = negotiator.negotiate(responses);
        assert!(result.final_response.contains("Answer B"));
    }

    #[test]
    fn test_best_of_n_strategy() {
        let negotiator = Negotiator::new(NegotiationStrategy::BestOfN);

        let responses = vec![
            ModelResponse {
                model_id: "model-a".to_string(),
                content: "Good answer".to_string(),
                confidence: Some(0.9),
                reasoning: None,
            },
            ModelResponse {
                model_id: "model-b".to_string(),
                content: "Better answer".to_string(),
                confidence: Some(0.95),
                reasoning: None,
            },
            ModelResponse {
                model_id: "model-c".to_string(),
                content: "Worst answer".to_string(),
                confidence: Some(0.5),
                reasoning: None,
            },
        ];

        let result = negotiator.negotiate(responses);
        assert!(result.final_response.contains("Better answer"));
    }
}
