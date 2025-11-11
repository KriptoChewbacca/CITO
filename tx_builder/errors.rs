//! Error types for TransactionBuilder

use solana_sdk::pubkey::Pubkey;
use thiserror::Error;

use crate::nonce_manager::NonceError;
use crate::rpc_manager::rpc_errors::RpcManagerError;

/// TransactionBuilder errors (Universe Class Enhanced)
#[derive(Debug, Clone, Error)]
pub enum TransactionBuilderError {
    #[error("Configuration validation failed: {0}")]
    ConfigValidation(String),
    
    #[error("RPC connection failed: {0}")]
    RpcConnection(String),
    
    #[error("RPC manager error: {0}")]
    RpcManager(#[from] RpcManagerError),
    
    #[error("Instruction building failed for {program}: {reason}")]
    InstructionBuild { program: String, reason: String },
    
    #[error("Signing failed: {0}")]
    SigningFailed(String),
    
    #[error("Blockhash fetch failed: {0}")]
    BlockhashFetch(String),
    
    #[error("Nonce error: {0}")]
    Nonce(#[from] NonceError),
    
    #[error("Serialization failed: {0}")]
    Serialization(String),
    
    #[error("Program {0} is not allowed by configuration")]
    ProgramNotAllowed(Pubkey),
    
    #[error("Feature not enabled: {feature} for {action}")]
    FeatureNotEnabled { feature: String, action: String },
    
    #[error("Simulation failed: {0}")]
    SimulationFailed(String),
    
    #[error("Insufficient balance: required {required}, available {available}")]
    InsufficientBalance { required: u64, available: u64 },
    
    #[error("Liquidity depth too low: {available} < {required}")]
    LiquidityTooLow { available: u64, required: u64 },
    
    #[error("Universe error: {0:?}")]
    Universe(UniverseErrorType),
}

/// Universe-level error classification (Universe Class)
#[derive(Debug, Clone)]
pub enum UniverseErrorType {
    TransientError { reason: String, retry_after_ms: u64 },
    FatalError { reason: String },
    SecurityViolation { reason: String, confidence: f64 },
    ComputeOverrun { used: u32, limit: u32 },
    AnomalyDetected { description: String, confidence: f64 },
}

/// Supported DEX programs with priority ordering (Universe Class Enhanced)
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum DexProgram {
    PumpFun,     // Priority 0 (highest)
    Raydium,     // Priority 1
    Orca,        // Priority 2
    LetsBonk,    // Priority 3
    Unknown(String), // Priority 4 (lowest)
}

impl DexProgram {
    /// Get priority score (lower is better) (Universe Class)
    pub fn priority(&self) -> u8 {
        match self {
            DexProgram::PumpFun => 0,
            DexProgram::Raydium => 1,
            DexProgram::Orca => 2,
            DexProgram::LetsBonk => 3,
            DexProgram::Unknown(_) => 255,
        }
    }
}

impl From<&str> for DexProgram {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "pump.fun" | "pumpfun" | "pumpportal" => DexProgram::PumpFun,
            "letsbonk.fun" | "letsbonk" | "bonk" => DexProgram::LetsBonk,
            "raydium" => DexProgram::Raydium,
            "orca" => DexProgram::Orca,
            _ => DexProgram::Unknown(s.to_string()),
        }
    }
}
