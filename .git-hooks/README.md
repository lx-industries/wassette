# Git Hooks

This directory contains git hooks that can be installed to enforce code quality and consistency checks before committing.

## Available Hooks

### pre-commit

The pre-commit hook runs `cargo +nightly fmt --all -- --check` to ensure all Rust code is properly formatted before allowing a commit.

If the check fails, the commit will be rejected with a message instructing you to format the code using:

```bash
cargo +nightly fmt --all
```

## Installation

To install these hooks, run from the repository root:

```bash
just install-git-hooks
```

Or use the installation script directly:

```bash
./scripts/install-git-hooks.sh
```

This will copy all hooks from `.git-hooks/` to `.git/hooks/` and make them executable.

## Manual Installation

You can also manually copy individual hooks:

```bash
cp .git-hooks/pre-commit .git/hooks/pre-commit
chmod +x .git/hooks/pre-commit
```

## Bypassing Hooks (Not Recommended)

In rare cases where you need to bypass the pre-commit hook, you can use:

```bash
git commit --no-verify
```

**Warning:** This is not recommended and should only be used in exceptional circumstances. All code must be properly formatted before being merged.
