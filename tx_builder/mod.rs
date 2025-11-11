//! tx_builder module - Production-ready TransactionBuilder for Solana sniper bot
//!
//! This module has been split into smaller, focused sub-modules for better maintainability:
//! - `config`: Configuration types (TransactionConfig, QuorumConfig, etc.)
//! - `errors`: Error types (TransactionBuilderError, UniverseErrorType, etc.)
//! - `rate_limit`: Rate limiting and circuit breaker implementations
//! - `types`: Core types (TxBuildOutput, ExecutionContext, etc.)
//! - `builder`: Main TransactionBuilder implementation
//! - `dex`: DEX-specific instruction builders

pub mod config;
pub mod errors;
pub mod rate_limit;
pub mod types;

// Re-export commonly used types for backward compatibility
pub use config::{QuorumConfig, SimulationCacheConfig, TransactionConfig};
pub use errors::{DexProgram, TransactionBuilderError, UniverseErrorType};
pub use rate_limit::{CircuitBreaker, CircuitBreakerStatus, CircuitState, RetryPolicy, TokenBucket};
pub use types::{
    ExecutionContext, JitoBundleCandidate, OperationPriority, ProgramMetadata,
    SlippagePredictor, SimulationCacheEntry, TxBuildOutput, TxBuilderMetrics,
};
