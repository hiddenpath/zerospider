//! Parallel task execution patterns.
//!
//! Implements various parallel execution modes:
//! - Map-Reduce: Parallel processing with aggregation
//! - Fan-out: Broadcast to multiple models
//! - Race: First valid response wins
//! - Pipeline: Sequential stages with parallel steps

use futures_util::{stream, StreamExt};
use std::sync::Arc;
use tokio::sync::{mpsc, Semaphore};

pub struct ParallelExecutor {
    concurrency_limit: usize,
    timeout_ms: u64,
}

impl ParallelExecutor {
    pub fn new(concurrency_limit: usize, timeout_ms: u64) -> Self {
        Self {
            concurrency_limit,
            timeout_ms,
        }
    }
}

impl Default for ParallelExecutor {
    fn default() -> Self {
        Self::new(10, 30000)
    }
}

#[derive(Debug, Clone)]
pub struct Task<T> {
    pub id: String,
    pub input: T,
}

#[derive(Debug)]
pub struct TaskResult<R> {
    pub task_id: String,
    pub result: anyhow::Result<R>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParallelMode {
    MapReduce,
    FanOut,
    Race,
    Pipeline,
}

pub trait TaskProcessor<T, R>: Send + Sync {
    fn process(&self, input: T) -> impl std::future::Future<Output = anyhow::Result<R>> + Send;
}

impl ParallelExecutor {
    pub async fn map_reduce<T, R, P, A>(
        &self,
        tasks: Vec<Task<T>>,
        processor: Arc<P>,
        aggregator: A,
    ) -> anyhow::Result<R>
    where
        T: Send + 'static,
        R: Send + 'static,
        P: TaskProcessor<T, R>,
        A: Fn(Vec<anyhow::Result<R>>) -> R + Send,
    {
        let semaphore = Arc::new(Semaphore::new(self.concurrency_limit));
        let timeout = std::time::Duration::from_millis(self.timeout_ms);

        let results: Vec<_> = stream::iter(tasks)
            .map(|task| {
                let processor = processor.clone();
                let semaphore = semaphore.clone();
                async move {
                    let _permit = semaphore.acquire().await.unwrap();
                    let result = tokio::time::timeout(timeout, processor.process(task.input)).await;

                    TaskResult::<R> {
                        task_id: task.id,
                        result: result
                            .map_err(|e| anyhow::anyhow!("Timeout: {}", e))
                            .and_then(|r| r),
                    }
                }
            })
            .buffer_unordered(self.concurrency_limit)
            .collect()
            .await;

        let outcomes: Vec<_> = results.into_iter().map(|r| r.result).collect();
        Ok(aggregator(outcomes))
    }

    pub async fn fan_out<T, R, P>(&self, input: T, processors: Vec<Arc<P>>) -> Vec<TaskResult<R>>
    where
        T: Clone + Send + 'static,
        R: Send + 'static,
        P: TaskProcessor<T, R>,
    {
        let semaphore = Arc::new(Semaphore::new(self.concurrency_limit));
        let timeout = std::time::Duration::from_millis(self.timeout_ms);

        let results: Vec<_> = stream::iter(processors.into_iter().enumerate())
            .map(|(idx, processor)| {
                let input = input.clone();
                let semaphore = semaphore.clone();
                async move {
                    let _permit = semaphore.acquire().await.unwrap();
                    let result = tokio::time::timeout(timeout, processor.process(input)).await;

                    TaskResult::<R> {
                        task_id: format!("processor-{}", idx),
                        result: result
                            .map_err(|e| anyhow::anyhow!("Timeout: {}", e))
                            .and_then(|r| r),
                    }
                }
            })
            .buffer_unordered(self.concurrency_limit)
            .collect()
            .await;

        results
    }

    pub async fn race<T, R, P>(&self, input: T, processors: Vec<Arc<P>>) -> anyhow::Result<R>
    where
        T: Clone + Send + 'static,
        R: Send + 'static,
        P: TaskProcessor<T, R> + 'static,
    {
        let (tx, mut rx) = mpsc::channel::<anyhow::Result<R>>(1);
        let timeout = std::time::Duration::from_millis(self.timeout_ms);

        for processor in processors {
            let input = input.clone();
            let tx = tx.clone();

            tokio::spawn(async move {
                let result = tokio::time::timeout(timeout, processor.process(input))
                    .await
                    .map_err(|e| anyhow::anyhow!("Timeout: {}", e))
                    .and_then(|r| r);

                let _ = tx.send(result).await;
            });
        }

        drop(tx);

        while let Some(result) = rx.recv().await {
            if result.is_ok() {
                return result;
            }
        }

        Err(anyhow::anyhow!("All processors failed"))
    }

    pub async fn pipeline<T, R, Stages>(&self, _input: T, _stages: Stages) -> anyhow::Result<R>
    where
        Stages: Send,
    {
        Err(anyhow::anyhow!(
            "Pipeline mode requires custom implementation"
        ))
    }
}

pub struct BatchProcessor<F>(pub F);

impl<T, R, F> TaskProcessor<T, R> for BatchProcessor<F>
where
    F: Fn(T) -> anyhow::Result<R> + Send + Sync,
    T: Send,
    R: Send,
{
    async fn process(&self, input: T) -> anyhow::Result<R> {
        (self.0)(input)
    }
}

pub struct AsyncBatchProcessor<F>(pub F);

impl<T, R, F, Fut> TaskProcessor<T, R> for AsyncBatchProcessor<F>
where
    F: Fn(T) -> Fut + Send + Sync,
    Fut: std::future::Future<Output = anyhow::Result<R>> + Send,
    T: Send,
    R: Send,
{
    async fn process(&self, input: T) -> anyhow::Result<R> {
        (self.0)(input).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Doubler;

    impl TaskProcessor<i32, i32> for Doubler {
        async fn process(&self, input: i32) -> anyhow::Result<i32> {
            Ok(input * 2)
        }
    }

    #[tokio::test]
    async fn test_map_reduce() {
        let executor = ParallelExecutor::new(5, 5000);
        let processor = Arc::new(Doubler);

        let tasks: Vec<Task<i32>> = vec![
            Task {
                id: "1".to_string(),
                input: 1,
            },
            Task {
                id: "2".to_string(),
                input: 2,
            },
            Task {
                id: "3".to_string(),
                input: 3,
            },
        ];

        let result = executor
            .map_reduce(tasks, processor, |results| {
                results.into_iter().filter_map(|r| r.ok()).sum()
            })
            .await;

        assert_eq!(result.unwrap(), 12);
    }

    #[tokio::test]
    async fn test_race() {
        let executor = ParallelExecutor::new(5, 5000);

        let processors: Vec<Arc<Doubler>> = vec![Arc::new(Doubler), Arc::new(Doubler)];

        let result = executor.race(21, processors).await;
        assert_eq!(result.unwrap(), 42);
    }
}
