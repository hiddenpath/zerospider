#[allow(clippy::module_inception)]
pub mod agent;
pub mod classifier;
pub mod dispatcher;
pub mod loop_;
pub mod memory_loader;
pub mod negotiation;
pub mod parallel;
pub mod prompt;

#[cfg(test)]
mod tests;

#[allow(unused_imports)]
pub use agent::{Agent, AgentBuilder};
#[allow(unused_imports)]
pub use loop_::{process_message, run};
#[allow(unused_imports)]
pub use negotiation::{ModelResponse, NegotiationResult, NegotiationStrategy, Negotiator};
#[allow(unused_imports)]
pub use parallel::{ParallelExecutor, ParallelMode, Task, TaskProcessor, TaskResult};
