# State Persistence Examples

This document provides practical examples of using Wassette's state persistence system.

## Basic Export and Import

### Export Current State

```rust
use wassette::{LifecycleManager, SnapshotOptions};

async fn export_basic() -> anyhow::Result<()> {
    let manager = LifecycleManager::new("/path/to/components").await?;
    
    // Export all components without binaries
    let options = SnapshotOptions::default();
    let snapshot = manager.export_state(options).await?;
    
    // Save to file
    snapshot.save_to_file("wassette-state.json").await?;
    
    Ok(())
}
```

### Import Saved State

```rust
use wassette::{LifecycleManager, RestoreOptions, StateSnapshot};

async fn import_basic() -> anyhow::Result<()> {
    let manager = LifecycleManager::new("/path/to/components").await?;
    
    // Load snapshot from file
    let snapshot = StateSnapshot::load_from_file("wassette-state.json").await?;
    
    // Import all components
    let options = RestoreOptions::default();
    let count = manager.import_state(&snapshot, options).await?;
    
    println!("Restored {} component(s)", count);
    Ok(())
}
```

## Common Use Cases

See the [State Persistence Design](../design/state-persistence.md) document for detailed workflow examples including:

- **Agent Handoff**: Transfer state between AI agents
- **Environment Migration**: Move from dev to staging to production
- **Backup and Restore**: Regular backups for disaster recovery
- **Team Collaboration**: Share configurations between developers
- **CI/CD Integration**: Consistent test environments

## Quick Reference

### Export Options

```rust
SnapshotOptions {
    include_binaries: bool,              // Include .wasm files (default: false)
    include_secrets: bool,               // Include secrets (default: false, NYI)
    encryption_key: Option<String>,      // For secret encryption (NYI)
    component_filter: Option<Vec<String>>, // Filter components
    metadata: Option<SnapshotMetadata>,  // Add custom metadata
}
```

### Import Options

```rust
RestoreOptions {
    skip_existing: bool,                 // Skip existing components (default: false)
    decryption_key: Option<String>,      // For secret decryption (NYI)
    component_filter: Option<Vec<String>>, // Filter components
    verify_checksums: bool,              // Verify integrity (default: false, NYI)
}
```

For complete examples and best practices, see the [design document](../design/state-persistence.md).
