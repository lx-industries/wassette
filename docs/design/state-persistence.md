# State Persistence Design

## Overview

This document describes the state persistence system for Wassette, which enables transferring the runtime state between different agents or environments. This addresses Issue #309 and builds upon the headless deployment mode from Issue #307.

## Motivation

### Use Cases

1. **Agent Handoff**: Transfer a running Wassette instance from one AI agent to another
   - Developer starts work with GitHub Copilot
   - Hands off to Claude Code for different perspective
   - Both agents share the same component state and permissions

2. **Backup and Restore**: Save Wassette state for disaster recovery
   - Regular snapshots of production configuration
   - Quick recovery from component corruption
   - Version control for infrastructure as code

3. **Environment Migration**: Move state between development, staging, and production
   - Develop and test in local environment
   - Export validated state to staging
   - Promote tested configuration to production

4. **Team Collaboration**: Share working configurations between team members
   - Developer A sets up complex component configuration
   - Export snapshot to git repository
   - Developer B imports and continues work

5. **CI/CD Integration**: Pre-configure Wassette for automated workflows
   - Export state from development environment
   - Import in CI pipeline for testing
   - Consistent test environment across runs

## Architecture

### State Components

Wassette state consists of several key components:

```
StateSnapshot
├── Version (schema compatibility)
├── Timestamp (creation time)
├── Metadata (description, tags, source)
└── Components[]
    ├── Component ID
    ├── Source URI
    ├── Metadata (tools, schemas, validation stamps)
    ├── Policy (permissions configuration)
    └── Binary Data (optional, base64 encoded)
```

### Data Flow

#### Export Flow
```
LifecycleManager
  ├── List Components
  ├── For Each Component:
  │   ├── Load Metadata (tools, schemas)
  │   ├── Load Policy (if exists)
  │   ├── Load Binary (if requested)
  │   └── Create ComponentState
  └── Serialize to JSON
      └── StateSnapshot
```

#### Import Flow
```
StateSnapshot
  ├── Validate Structure
  ├── Filter Components (if requested)
  ├── For Each Component:
  │   ├── Check if exists (skip if requested)
  │   ├── Restore Policy File
  │   ├── Restore Binary (if included)
  │   ├── Restore Metadata
  │   └── Load Component (if binary present)
  └── Return restoration count
```

## API Design

### Core Types

```rust
pub struct StateSnapshot {
    pub version: u32,
    pub created_at: u64,
    pub components: Vec<ComponentState>,
    pub metadata: Option<SnapshotMetadata>,
}

pub struct ComponentState {
    pub component_id: String,
    pub source_uri: String,
    pub metadata: ComponentMetadata,
    pub policy: Option<PolicyState>,
    pub binary_data: Option<String>,  // Base64 encoded
    pub include_binary: Option<bool>,
}

pub struct SnapshotOptions {
    pub include_binaries: bool,
    pub include_secrets: bool,
    pub encryption_key: Option<String>,
    pub component_filter: Option<Vec<String>>,
    pub metadata: Option<SnapshotMetadata>,
}

pub struct RestoreOptions {
    pub skip_existing: bool,
    pub decryption_key: Option<String>,
    pub component_filter: Option<Vec<String>>,
    pub verify_checksums: bool,
}
```

### Methods

```rust
impl LifecycleManager {
    pub async fn export_state(
        &self,
        options: SnapshotOptions,
    ) -> Result<StateSnapshot>;
    
    pub async fn import_state(
        &self,
        snapshot: &StateSnapshot,
        options: RestoreOptions,
    ) -> Result<usize>;
}

impl StateSnapshot {
    pub fn to_json(&self) -> Result<String>;
    pub fn from_json(json: &str) -> Result<Self>;
    pub async fn save_to_file(&self, path: impl AsRef<Path>) -> Result<()>;
    pub async fn load_from_file(path: impl AsRef<Path>) -> Result<Self>;
    pub fn validate(&self) -> Result<()>;
}
```

## Security Considerations

### Secrets Handling

**Decision**: Secrets are NOT included in snapshots by default.

**Rationale**:
- Secrets are environment-specific (dev vs prod)
- Snapshot files may be committed to version control
- Risk of accidental exposure in logs or backups
- Secrets should be managed through proper secret management systems

**Future Work**: Optional encrypted secrets with explicit opt-in
```rust
SnapshotOptions {
    include_secrets: true,
    encryption_key: Some("encryption-key-from-env"),
    // Secrets encrypted with AES-256-GCM
}
```

### Permission Preservation

Policies ARE included in snapshots because:
- They define security boundaries
- They're declarative and auditable
- They're required for component operation
- They're not sensitive like secrets

### File Permissions

On Unix systems, restored policy files get 0600 permissions (owner read/write only) to prevent unauthorized access.

## Data Format

### JSON Schema

Example snapshot structure:

```json
{
  "version": 1,
  "created_at": 1731444000,
  "metadata": {
    "description": "Development environment snapshot",
    "wassette_version": "0.3.4",
    "source": "developer-laptop",
    "tags": {
      "environment": "development",
      "project": "ai-assistant"
    }
  },
  "components": [
    {
      "component_id": "fetch-rs",
      "source_uri": "oci://ghcr.io/microsoft/fetch-rs:latest",
      "metadata": {
        "component_id": "fetch-rs",
        "tool_schemas": [...],
        "function_identifiers": [...],
        "tool_names": ["fetch"],
        "validation_stamp": {
          "file_size": 1234567,
          "mtime": 1731443000,
          "content_hash": "sha256:abcd..."
        },
        "created_at": 1731443000
      },
      "policy": {
        "content": "network:\n  allow:\n    - host: api.github.com\n",
        "source_uri": "inline",
        "created_at": 1731443000
      },
      "include_binary": false
    }
  ]
}
```

### Versioning

- **version**: Schema version (currently 1)
- Future versions may add fields but maintain backward compatibility
- Old versions should gracefully handle unknown fields
- Breaking changes require major version bump

## Performance Considerations

### Binary Inclusion

**With Binaries** (include_binaries: true):
- **Pros**: Self-contained snapshots, offline restore capability
- **Cons**: Large file sizes (components can be 1-10MB each)
- **Use Case**: Cross-environment deployment, air-gapped systems

**Without Binaries** (include_binaries: false, default):
- **Pros**: Small snapshots (<1KB per component), git-friendly
- **Cons**: Requires network access to re-download components
- **Use Case**: Configuration backup, team collaboration

### Component Filtering

Filter by component ID to export/import specific components:
```rust
SnapshotOptions {
    component_filter: Some(vec![
        "fetch-rs".to_string(),
        "filesystem-rs".to_string(),
    ]),
    ..Default::default()
}
```

This reduces snapshot size and import time for large installations.

## Consistency Guarantees

### Export Consistency

Snapshots capture a point-in-time view of the component registry. During export:
- Component list is enumerated once
- Each component state is read atomically
- No locks are held across components
- Concurrent component loads may not be reflected

### Import Atomicity

Import is performed component-by-component:
- Each component restore is independent
- Partial failures leave some components restored
- Failed components are logged but don't block others
- Return value indicates number of successful restorations

## Future Enhancements

### 1. Incremental Snapshots
Track component versions and only export changes since last snapshot.

### 2. Encrypted Secrets
Add optional AES-256-GCM encryption for secrets with key derivation:
```rust
SnapshotOptions {
    include_secrets: true,
    encryption_key: Some(derive_key_from_passphrase("user-passphrase")),
}
```

### 3. Snapshot Compression
Compress snapshots with gzip/zstd for large binary-included snapshots.

### 4. Remote Storage
Built-in support for S3/Azure/GCS storage backends:
```rust
snapshot.save_to_remote("s3://bucket/snapshots/prod.json").await?;
```

### 5. Checksum Verification
Verify component integrity during restore:
```rust
RestoreOptions {
    verify_checksums: true,  // Check SHA-256 hashes
}
```

### 6. Diff and Merge
Compare snapshots and selectively merge components:
```rust
let diff = snapshot1.diff(&snapshot2);
let merged = snapshot1.merge(&snapshot2, MergeStrategy::Newer);
```

### 7. State Locking
Prevent concurrent state modifications during export:
```rust
let _lock = lifecycle_manager.acquire_state_lock().await?;
let snapshot = lifecycle_manager.export_state(options).await?;
```

## Integration with Headless Mode

State persistence complements the headless deployment mode (Issue #307):

### Headless Manifest → State Snapshot
```bash
# Start with manifest
wassette serve --manifest deployment.yaml

# Export resulting state
wassette state export --output deployment-state.json

# Version control the state
git add deployment-state.json
```

### State Snapshot → Component Loading
```bash
# Import state
wassette state import deployment-state.json

# Components are loaded and ready
# Equivalent to manifest provisioning
```

### Combined Workflow
```yaml
# deployment.yaml
version: 1
components:
  - uri: oci://ghcr.io/microsoft/fetch-rs:v1.0.0
    permissions:
      network:
        allow:
          - host: api.github.com
```

After provisioning, export for team sharing:
```bash
wassette serve --manifest deployment.yaml &
wassette state export --output team-config.json
```

Team member imports:
```bash
wassette state import team-config.json
# Now has identical configuration
```

## Testing Strategy

### Unit Tests
- ✅ Snapshot creation and validation
- ✅ JSON serialization/deserialization
- ✅ File I/O operations
- ✅ Duplicate component detection

### Integration Tests (To Do)
- [ ] Export with real components
- [ ] Import and verify component functionality
- [ ] Binary inclusion/exclusion
- [ ] Component filtering
- [ ] Skip existing behavior
- [ ] Cross-version compatibility

### End-to-End Tests (To Do)
- [ ] Agent handoff scenario
- [ ] Environment migration
- [ ] Backup and restore
- [ ] Team collaboration workflow

## CLI Interface (Proposed)

```bash
# Export state
wassette state export [OPTIONS]
  --output <FILE>           Output file path (default: wassette-state.json)
  --include-binaries        Include component binaries
  --components <IDS>        Filter by component IDs (comma-separated)
  --description <TEXT>      Snapshot description
  --tag <KEY=VALUE>         Add metadata tags (can be repeated)

# Import state  
wassette state import <FILE> [OPTIONS]
  --skip-existing           Skip components that already exist
  --components <IDS>        Filter by component IDs
  --verify                  Verify checksums before restore

# List snapshots
wassette state list
  --format <json|table>     Output format

# Inspect snapshot
wassette state inspect <FILE>
  --show-binaries           Show binary data presence
  --show-policies           Show policy details
```

## Comparison with Other Approaches

### vs. Manifest-Based Provisioning
| Aspect | State Snapshot | Manifest |
|--------|---------------|----------|
| Source | Runtime state | Declarative config |
| Completeness | Exact runtime state | Intent-based |
| Binary inclusion | Optional | Never |
| Use case | Migration, backup | Initial setup |
| Metadata | Captured | Minimal |

### vs. Container Images
| Aspect | State Snapshot | Container Image |
|--------|---------------|-----------------|
| Portability | High (JSON) | Medium (registry) |
| Size | Small without binaries | Large |
| Dependencies | Components separate | Bundled |
| Version control | Git-friendly | Registry-based |

### vs. Database Dumps
| Aspect | State Snapshot | Database Dump |
|--------|---------------|---------------|
| Format | Structured JSON | Binary/SQL |
| Human-readable | Yes | No |
| Partial restore | Yes (filtering) | Limited |
| Secrets | Excluded | Included |

## Conclusion

The state persistence system provides a flexible, secure, and efficient way to capture and transfer Wassette runtime state. It complements the headless deployment mode by enabling state-based workflows alongside manifest-based provisioning. The JSON-based format ensures compatibility with version control systems and CI/CD pipelines while maintaining human readability.

Key benefits:
- **Flexibility**: Optional binary inclusion, component filtering, metadata tags
- **Security**: Secrets excluded by default, proper file permissions
- **Compatibility**: Git-friendly JSON format, version tracking
- **Performance**: Small snapshots without binaries, efficient restore
- **Extensibility**: Clear path for encrypted secrets, compression, remote storage

The implementation is ready for CLI integration and real-world testing with production components.
