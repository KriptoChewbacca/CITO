//! Configuration types for transaction building

use dashmap::DashMap;
use solana_sdk::pubkey::Pubkey;
use std::sync::Arc;

use super::errors::{DexProgram, TransactionBuilderError};
use super::rate_limit::RetryPolicy;
use super::types::{OperationPriority, ProgramMetadata};

#[cfg(feature = "pumpfun")]
use pumpfun::common::types::Cluster;

/// Quorum configuration for blockhash consensus
#[derive(Debug, Clone)]
pub struct QuorumConfig {
    /// Minimum number of RPC responses required for quorum
    pub min_responses: usize,
    /// Maximum allowed slot difference between responses
    pub max_slot_diff: u64,
    /// Enable slot-based validation
    pub enable_slot_validation: bool,
}

impl Default for QuorumConfig {
    fn default() -> Self {
        Self {
            min_responses: 2,
            max_slot_diff: 10,
            enable_slot_validation: true,
        }
    }
}

/// Configuration for simulation caching
#[derive(Debug, Clone)]
pub struct SimulationCacheConfig {
    /// Time-to-live for cached simulation results
    pub ttl_seconds: u64,
    /// Maximum cache size (number of entries)
    pub max_size: usize,
    /// Enable simulation caching
    pub enabled: bool,
    /// Programs to exclude from caching (by program_id string)
    pub excluded_programs: Vec<String>,
}

impl Default for SimulationCacheConfig {
    fn default() -> Self {
        Self {
            ttl_seconds: 30,
            max_size: 1000,
            enabled: true,
            excluded_programs: Vec::new(),
        }
    }
}

impl SimulationCacheConfig {
    /// Check if a program should be excluded from caching
    pub fn is_program_excluded(&self, program_id: &Pubkey) -> bool {
        let program_str = program_id.to_string();
        self.excluded_programs.iter().any(|excluded| excluded == &program_str)
    }
}

/// Transaction configuration with Universe Class enhancements
#[derive(Debug, Clone)]
pub struct TransactionConfig {
    pub priority_fee_lamports: u64,
    pub compute_unit_limit: u32,
    pub min_cu_limit: u32,
    pub max_cu_limit: u32,
    pub adaptive_priority_fee_base: u64,
    pub adaptive_priority_fee_multiplier: f64,
    pub buy_amount_lamports: u64,
    pub slippage_bps: u64,
    pub rpc_endpoints: Arc<[String]>,
    pub rpc_retry_attempts: usize,
    pub rpc_timeout_ms: u64,
    pub pumpportal_url: Option<String>,
    pub pumpportal_api_key: Option<String>,
    pub letsbonk_api_url: Option<String>,
    pub letsbonk_api_key: Option<String>,
    pub jito_bundle_enabled: bool,
    pub signer_keypair_index: Option<usize>,
    pub nonce_count: usize,
    pub allowed_programs: Arc<DashMap<Pubkey, ProgramMetadata>>,
    pub dex_priority: Vec<DexProgram>,
    pub min_liquidity_lamports: u64,
    pub enable_simulation: bool,
    pub enable_ml_slippage: bool,
    pub quorum_config: QuorumConfig,
    pub retry_policy: RetryPolicy,
    pub rpc_rate_limit_rps: f64,
    pub simulation_rate_limit_rps: f64,
    pub http_rate_limit_rps: f64,
    pub circuit_breaker_failure_threshold: u32,
    pub circuit_breaker_timeout_secs: u64,
    pub simulation_cache_config: SimulationCacheConfig,
    pub max_concurrent_builds: usize,
    pub operation_priority: OperationPriority,
    pub signer_rotation_interval: u64,
    #[cfg(feature = "pumpfun")]
    pub cluster: Cluster,
}

impl Default for TransactionConfig {
    fn default() -> Self {
        Self {
            priority_fee_lamports: 10_000,
            compute_unit_limit: 200_000,
            min_cu_limit: 100_000,
            max_cu_limit: 400_000,
            adaptive_priority_fee_base: 10_000,
            adaptive_priority_fee_multiplier: 1.5,
            buy_amount_lamports: 10_000_000,
            slippage_bps: 1000,
            rpc_endpoints: Arc::new(["https://api.mainnet-beta.solana.com".to_string()]),
            rpc_retry_attempts: 3,
            rpc_timeout_ms: 8_000,
            pumpportal_url: None,
            pumpportal_api_key: None,
            letsbonk_api_url: None,
            letsbonk_api_key: None,
            jito_bundle_enabled: false,
            signer_keypair_index: None,
            nonce_count: 5,
            allowed_programs: Arc::new(DashMap::new()),
            dex_priority: vec![DexProgram::PumpFun, DexProgram::Raydium, DexProgram::Orca],
            min_liquidity_lamports: 1_000_000_000,
            enable_simulation: true,
            enable_ml_slippage: false,
            quorum_config: QuorumConfig::default(),
            retry_policy: RetryPolicy::default(),
            rpc_rate_limit_rps: 100.0,
            simulation_rate_limit_rps: 20.0,
            http_rate_limit_rps: 50.0,
            circuit_breaker_failure_threshold: 5,
            circuit_breaker_timeout_secs: 60,
            simulation_cache_config: SimulationCacheConfig::default(),
            max_concurrent_builds: 50,
            operation_priority: OperationPriority::default(),
            signer_rotation_interval: 100,
            #[cfg(feature = "pumpfun")]
            cluster: Cluster::mainnet(Default::default(), Default::default()),
        }
    }
}

impl TransactionConfig {
    pub fn validate(&self) -> Result<(), TransactionBuilderError> {
        if self.buy_amount_lamports == 0 {
            return Err(TransactionBuilderError::ConfigValidation(
                "buy_amount_lamports must be > 0".to_string(),
            ));
        }
        if self.slippage_bps > 10000 {
            return Err(TransactionBuilderError::ConfigValidation(
                "slippage_bps must be <= 10000".to_string(),
            ));
        }
        if self.rpc_endpoints.is_empty() {
            return Err(TransactionBuilderError::ConfigValidation(
                "rpc_endpoints must contain at least one endpoint".to_string(),
            ));
        }
        if self.nonce_count == 0 {
            return Err(TransactionBuilderError::ConfigValidation(
                "nonce_count must be > 0".to_string(),
            ));
        }
        if self.min_cu_limit > self.max_cu_limit {
            return Err(TransactionBuilderError::ConfigValidation(
                "min_cu_limit must be <= max_cu_limit".to_string(),
            ));
        }
        if self.adaptive_priority_fee_multiplier < 1.0 {
            return Err(TransactionBuilderError::ConfigValidation(
                "adaptive_priority_fee_multiplier must be >= 1.0".to_string(),
            ));
        }
        if self.quorum_config.min_responses == 0 {
            return Err(TransactionBuilderError::ConfigValidation(
                "quorum_config.min_responses must be > 0".to_string(),
            ));
        }
        if self.quorum_config.min_responses > self.rpc_endpoints.len() {
            return Err(TransactionBuilderError::ConfigValidation(
                format!("quorum_config.min_responses ({}) cannot exceed number of RPC endpoints ({})", 
                    self.quorum_config.min_responses, self.rpc_endpoints.len()),
            ));
        }
        if self.circuit_breaker_failure_threshold == 0 {
            return Err(TransactionBuilderError::ConfigValidation(
                "circuit_breaker_failure_threshold must be > 0".to_string(),
            ));
        }
        if self.max_concurrent_builds == 0 {
            return Err(TransactionBuilderError::ConfigValidation(
                "max_concurrent_builds must be > 0".to_string(),
            ));
        }
        Ok(())
    }

    pub fn is_program_allowed(&self, program_id: &Pubkey) -> bool {
        self.allowed_programs.is_empty() || self.allowed_programs.contains_key(program_id)
    }
    
    pub fn add_allowed_program(&self, program_id: Pubkey, metadata: ProgramMetadata) {
        self.allowed_programs.insert(program_id, metadata);
    }
    
    pub fn get_program_metadata(&self, program_id: &Pubkey) -> Option<ProgramMetadata> {
        self.allowed_programs.get(program_id).map(|r| r.clone())
    }
    
    pub fn calculate_adaptive_priority_fee(&self) -> u64 {
        (self.adaptive_priority_fee_base as f64 * self.adaptive_priority_fee_multiplier) as u64
    }
}
