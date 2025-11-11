//! Core types and data structures for transaction building

use crate::nonce_manager::NonceLease;
use serde::{Deserialize, Serialize};
use solana_sdk::{
    hash::Hash,
    pubkey::Pubkey,
    transaction::VersionedTransaction,
};
use std::collections::VecDeque;
use std::time::Instant;
use tracing::warn;

use super::errors::TransactionBuilderError;

// ============================================================================
// Transaction Build Output (Phase 1: RAII Nonce Management)
// ============================================================================

/// Output from transaction building with nonce lease management (RAII pattern)
/// 
/// This struct ensures proper lifecycle management of nonce leases through RAII:
/// - Holds the built transaction ready for signing/broadcast
/// - Maintains ownership of the nonce lease until explicitly released or dropped
/// - Automatically warns if lease is not properly released before drop
/// - Extracts required signers from transaction header for validation
/// 
/// # RAII Contract
/// 
/// This struct enforces the following RAII guarantees:
/// 
/// 1. **Owned Data**: All fields contain owned data ('static), no references
/// 2. **Automatic Cleanup**: `Drop` implementation ensures nonce lease is released
/// 3. **Explicit Release**: Prefer `release_nonce()` for controlled cleanup
/// 4. **Consume Pattern**: `release_nonce()` consumes `self` to prevent use-after-release
/// 5. **No Async in Drop**: Drop only logs; actual release is synchronous
/// 6. **Zero Leaks**: Lease is guaranteed to be released either explicitly or on drop
pub struct TxBuildOutput {
    /// The built transaction ready for signing/broadcast
    pub tx: VersionedTransaction,
    
    /// Optional nonce lease guard (held until broadcast completes)
    /// Automatically released on drop via RAII pattern
    pub nonce_guard: Option<NonceLease>,
    
    /// List of required signers for this transaction
    /// Extracted from message.header.num_required_signatures
    pub required_signers: Vec<Pubkey>,
}

impl TxBuildOutput {
    /// Create new TxBuildOutput with nonce guard
    pub fn new(
        tx: VersionedTransaction,
        nonce_guard: Option<NonceLease>,
    ) -> Self {
        // Extract required signers using compat layer for unified API
        let required_signers = crate::compat::get_required_signers(&tx.message)
            .to_vec();
        
        Self {
            tx,
            nonce_guard,
            required_signers,
        }
    }
    
    /// Explicitly release nonce guard (if held)
    pub async fn release_nonce(mut self) -> Result<(), TransactionBuilderError> {
        if let Some(guard) = self.nonce_guard.take() {
            guard.release().await?;
        }
        Ok(())
    }
}

impl Drop for TxBuildOutput {
    fn drop(&mut self) {
        if let Some(ref guard) = self.nonce_guard {
            warn!(
                nonce = %guard.nonce_pubkey(),
                drop_source = "TxBuildOutput",
                "TxBuildOutput dropped with active nonce guard - lease will be auto-released via NonceLease Drop"
            );
        }
    }
}

/// Simulation cache entry with TTL
#[derive(Debug, Clone)]
pub(crate) struct SimulationCacheEntry {
    pub(crate) compute_units: u64,
    pub(crate) cached_at: Instant,
    pub(crate) slot: u64,
}

/// Metadata for tracking program information (Universe Class)
#[derive(Debug, Clone)]
pub struct ProgramMetadata {
    /// Program version string (e.g., "1.0.0")
    pub version: String,
    /// Last slot where this program was verified
    pub last_verified_slot: u64,
    /// Whether the program has been verified as safe
    pub is_verified: bool,
}

impl Default for ProgramMetadata {
    fn default() -> Self {
        Self {
            version: "unknown".to_string(),
            last_verified_slot: 0,
            is_verified: false,
        }
    }
}

/// ML-based slippage predictor using recent market volatility (Universe Class)
#[derive(Debug)]
pub struct SlippagePredictor {
    history: VecDeque<f64>,
    max_history_size: usize,
}

impl SlippagePredictor {
    pub fn new(max_history_size: usize) -> Self {
        Self {
            history: VecDeque::with_capacity(max_history_size),
            max_history_size,
        }
    }

    pub fn add_observation(&mut self, bps: f64) {
        if self.history.len() >= self.max_history_size {
            self.history.pop_front();
        }
        self.history.push_back(bps);
    }

    pub fn predict_optimal_slippage(&self, base_bps: u64) -> u64 {
        if self.history.is_empty() {
            return base_bps;
        }
        
        // Calculate volatility (standard deviation)
        let mean: f64 = self.history.iter().sum::<f64>() / self.history.len() as f64;
        let variance: f64 = self.history.iter()
            .map(|x| (x - mean).powi(2))
            .sum::<f64>() / self.history.len() as f64;
        let std_dev = variance.sqrt();
        
        // Adjust slippage based on volatility (higher volatility = higher slippage)
        let multiplier = 1.0 + (std_dev / 100.0).min(0.5); // Cap at 50% increase
        ((base_bps as f64) * multiplier).round() as u64
    }
}

/// Execution context holding blockhash and optional nonce lease
pub(crate) struct ExecutionContext {
    pub(crate) blockhash: Hash,
    pub(crate) nonce_pubkey: Option<Pubkey>,
    pub(crate) nonce_authority: Option<Pubkey>,
    pub(crate) nonce_lease: Option<NonceLease>,
    #[cfg(feature = "zk_enabled")]
    pub(crate) zk_proof: Option<crate::nonce_manager::ZkProofData>,
}

impl std::fmt::Debug for ExecutionContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut debug_struct = f.debug_struct("ExecutionContext");
        debug_struct
            .field("blockhash", &self.blockhash)
            .field("nonce_pubkey", &self.nonce_pubkey)
            .field("nonce_authority", &self.nonce_authority)
            .field("nonce_lease_status", &match &self.nonce_lease {
                Some(lease) => format!("Some(nonce={}, expired={})", 
                    lease.nonce_pubkey(), 
                    lease.is_expired()),
                None => "None".to_string(),
            });
        
        #[cfg(feature = "zk_enabled")]
        debug_struct.field("zk_proof", &self.zk_proof.as_ref().map(|p| 
            format!("Present(confidence={:.2})", p.confidence)));
        
        debug_struct.finish()
    }
}

impl ExecutionContext {
    /// Extract the nonce lease, consuming it
    pub fn extract_lease(mut self) -> Option<NonceLease> {
        self.nonce_lease.take()
    }
}

/// Operation priority for nonce vs blockhash decision
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationPriority {
    /// Critical sniper operations - require lease, fail fast on exhaustion
    CriticalSniper,
    /// Utility operations - prefer recent blockhash for speed
    Utility,
    /// Bulk/non-urgent operations - use recent if nonce pool below threshold
    Bulk,
}

impl Default for OperationPriority {
    fn default() -> Self {
        OperationPriority::Utility
    }
}

impl OperationPriority {
    /// Check if this operation requires a nonce lease
    pub fn requires_nonce(&self) -> bool {
        match self {
            OperationPriority::CriticalSniper => true,
            OperationPriority::Utility => false,
            OperationPriority::Bulk => false,
        }
    }
    
    /// Check if fallback to recent blockhash is allowed on nonce exhaustion
    pub fn allow_blockhash_fallback(&self) -> bool {
        match self {
            OperationPriority::CriticalSniper => false,
            OperationPriority::Utility => true,
            OperationPriority::Bulk => true,
        }
    }
}

/// Jito bundle representation (Universe Class Enhanced)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JitoBundleCandidate {
    pub transactions: Vec<VersionedTransaction>,
    pub max_total_cost_lamports: u64,
    pub target_slot: Option<u64>,
    pub searcher_hints: Vec<u8>,
    pub backrun_protect: bool,
}

/// Telemetry metrics structure
#[derive(Debug, Clone)]
pub struct TxBuilderMetrics {
    pub simulation_cache_size: usize,
    pub simulation_cache_capacity: usize,
    pub simulation_cache_hits: u64,
    pub simulation_cache_misses: u64,
    pub nonce_acquire_count: u64,
    pub nonce_exhausted_count: u64,
    pub blockhash_quorum_success_count: u64,
    pub blockhash_fallback_count: u64,
    pub transaction_count: u64,
    pub rotation_checkpoint: u64,
}
