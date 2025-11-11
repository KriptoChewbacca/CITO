# tx_builder Module

This directory contains the modularized version of the `tx_builder.rs` file, organized into logical components for better maintainability.

## Module Structure

```
tx_builder/
├── mod.rs                    # Module entry point, re-exports public API
├── config.rs                 # Configuration types
│   ├── TransactionConfig     # Main configuration struct
│   ├── QuorumConfig          # Blockhash quorum settings
│   └── SimulationCacheConfig # Simulation cache settings
├── errors.rs                 # Error types
│   ├── TransactionBuilderError # Main error enum
│   ├── UniverseErrorType     # Universe-level error classification  
│   └── DexProgram            # DEX program enumeration
├── rate_limit.rs             # Rate limiting and resilience
│   ├── TokenBucket           # Token bucket rate limiter
│   ├── CircuitBreaker        # Circuit breaker for RPC endpoints
│   ├── CircuitState          # Circuit breaker states
│   └── RetryPolicy           # Retry policy with backoff
├── types.rs                  # Core types and data structures
│   ├── TxBuildOutput         # Transaction build output with RAII
│   ├── ExecutionContext      # Execution context for transactions
│   ├── SlippagePredictor     # ML-based slippage prediction
│   ├── ProgramMetadata       # Program verification metadata
│   ├── OperationPriority     # Operation priority levels
│   ├── JitoBundleCandidate   # Jito bundle representation
│   ├── TxBuilderMetrics      # Telemetry metrics
│   └── SimulationCacheEntry  # Simulation cache entry
└── dex/                      # DEX-specific implementations (future)
```

## Usage

### Importing from the module

```rust
// Import individual types
use tx_builder::config::TransactionConfig;
use tx_builder::errors::TransactionBuilderError;
use tx_builder::types::TxBuildOutput;

// Or use re-exports from mod.rs
use tx_builder::{TransactionConfig, TransactionBuilderError, TxBuildOutput};
```

### Current Status

The modularization is in progress:
- ✅ Core types extracted to separate files
- ✅ Module structure created with proper visibility
- ✅ Documentation added
- ⏳ Main `TransactionBuilder` implementation still in `tx_builder.rs`
- ⏳ DEX-specific code to be extracted to `dex/` subdirectory
- ⏳ Complete migration to use modularized code

## Benefits

1. **Better Organization**: Related code is grouped together
2. **Improved Maintainability**: Smaller, focused modules are easier to understand
3. **Clearer Dependencies**: Module boundaries make dependencies explicit
4. **Easier Testing**: Individual modules can be tested in isolation
5. **Better Code Navigation**: IDEs can better navigate modular structure

## Backward Compatibility

The original `tx_builder.rs` file remains functional and contains the full implementation.
Code using the old imports will continue to work without changes.

## Future Work

- Complete extraction of `TransactionBuilder` implementation to `builder.rs`
- Extract DEX instruction builders to `dex/` subdirectory
- Remove duplicate code from main `tx_builder.rs` file
- Add module-level tests
- Update all imports to use modularized structure
