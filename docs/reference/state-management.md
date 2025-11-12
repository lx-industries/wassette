# State Management

Wassette provides state management capabilities that allow you to export and import the complete state of your Wassette instance. This enables:

- **Agent Handoff**: Transfer state between different AI agents
- **Backup & Restore**: Snapshot your configuration for disaster recovery
- **Environment Migration**: Move component setups between dev/staging/prod
- **CI/CD Integration**: Pre-configure components for automated workflows

## State Components

When you export Wassette state, the following information is captured:

1. **Component Metadata**: All loaded components with their tool schemas, function identifiers, and validation stamps
2. **Policy Attachments**: Policy YAML content and source URIs for each component
3. **Secrets Configuration**: Environment variable keys (not values) for security
4. **Environment Variables**: Configuration environment variables
5. **Directory Paths**: Component and secrets directory locations

## Exporting State

Export the current Wassette state to a file:

```bash
wassette state export state-snapshot.json
```

The command supports both JSON and YAML formats based on the file extension:

```bash
# Export as JSON
wassette state export state.json

# Export as YAML
wassette state export state.yaml
```

You can specify custom component and secrets directories:

```bash
wassette state export state.json \
  --component-dir /path/to/components \
  --secrets-dir /path/to/secrets
```

## Importing State

Import a previously exported state snapshot:

```bash
wassette state import state-snapshot.json
```

The import process:

1. Validates state version compatibility
2. Checks for component WASM files at expected locations
3. Restores component metadata
4. Restores policy attachments and metadata
5. Notes which components require secret restoration

Components without WASM files are skipped with warnings logged.

## State File Format

The state file contains structured information about your Wassette instance:

```yaml
version: "0.3.4"
exported_at: 1699123456
components:
  - component_id: "example-component"
    metadata:
      tool_schemas: [...]
      function_identifiers: [...]
      validation_stamp:
        file_size: 123456
        mtime: 1699123450
    policy:
      source_uri: "file:///path/to/policy.yaml"
      policy_content: "..."
      attached_at: 1699123450
    secrets:
      env_keys: ["API_KEY", "SECRET_TOKEN"]
environment: {}
paths:
  component_dir: "/home/user/.local/share/wassette/components"
  secrets_dir: "/home/user/.config/wassette/secrets"
```

## Security Considerations

### Secrets Handling

For security reasons, **secret values are never exported**. The state snapshot only includes:

- Secret environment variable keys
- Indication that secrets exist for a component

After importing state, you must manually restore secrets:

```bash
# Restore secrets for a component
wassette secret set example-component API_KEY=your-key SECRET_TOKEN=your-token
```

### Policy Content

Policy YAML content IS included in the export to enable full restoration of permissions and access rules. Ensure your state snapshots are stored securely.

## Example Workflows

### Agent Handoff

Transfer state from one agent session to another:

```bash
# Agent 1: Export current state
wassette state export agent1-state.json

# Transfer file to Agent 2's environment

# Agent 2: Import state
wassette state import agent1-state.json

# Agent 2: Restore secrets
wassette secret set component1 KEY=value
```

### Backup Before Changes

Create a snapshot before making significant changes:

```bash
# Backup current state
wassette state export backup-$(date +%Y%m%d).json

# Make changes to components, policies, etc.

# If needed, restore from backup
wassette state import backup-20241112.json
```

### Environment Promotion

Promote configuration from dev to production:

```bash
# Development environment
wassette state export dev-config.yaml

# Copy to production environment

# Production environment
wassette state import dev-config.yaml
# Restore production-specific secrets
wassette secret set component1 PROD_API_KEY=prod-value
```

## Limitations

1. **Component WASM Files Required**: The import process expects component WASM files to already exist at the configured component directory. Components are not automatically fetched.

2. **Manual Secret Restoration**: Secret values must be manually set after import for security.

3. **Environment-Specific Paths**: Path configurations in the state file may need adjustment when moving between environments.

## Future Enhancements

This state management implementation lays the groundwork for more advanced features:

- **Manifest-Based Provisioning**: Declarative component loading with automatic fetching
- **Deployment Profiles**: Separate configurations for interactive vs. headless modes
- **Auto-Fetching**: Automatically download missing components during import
- **Secret Integration**: Integration with external secret managers (HashiCorp Vault, etc.)

For more information about planned headless deployment features, see [Issue #307](https://github.com/microsoft/wassette/issues/307).
