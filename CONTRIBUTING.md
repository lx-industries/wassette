# Contributing

This project welcomes contributions and suggestions. Most contributions require you to
agree to a Contributor License Agreement (CLA) declaring that you have the right to,
and actually do, grant us the rights to use your contribution. For details, visit
https://cla.microsoft.com.

When you submit a pull request, a CLA-bot will automatically determine whether you need
to provide a CLA and decorate the PR appropriately (e.g., label, comment). Simply follow the
instructions provided by the bot. You will only need to do this once across all repositories using our CLA.

This project has adopted the [Microsoft Open Source Code of Conduct](https://opensource.microsoft.com/codeofconduct/).
For more information see the [Code of Conduct FAQ](https://opensource.microsoft.com/codeofconduct/faq/)
or contact [opencode@microsoft.com](mailto:opencode@microsoft.com) with any additional questions or comments.

## Development Setup

### Git Hooks

This repository includes pre-commit hooks to ensure code quality and consistency. To install the git hooks, run:

```bash
just install-git-hooks
```

Or directly:

```bash
./scripts/install-git-hooks.sh
```

The pre-commit hook will automatically run `cargo +nightly fmt --all -- --check` before each commit to ensure all Rust code is properly formatted. If the check fails, format your code with:

```bash
cargo +nightly fmt --all
```

Then stage the changes and commit again.

### Code Formatting

All Rust code must be formatted using `cargo +nightly fmt`. This is enforced by:
- Pre-commit hooks (if installed)
- CI/CD pipeline checks

To format your code manually:

```bash
cargo +nightly fmt --all
```

To check formatting without modifying files:

```bash
cargo +nightly fmt --all -- --check
```