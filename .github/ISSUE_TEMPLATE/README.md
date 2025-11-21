# Issue Templates

This directory contains issue templates for the Wassette repository. These templates help ensure that issues contain all necessary information for efficient triage and resolution.

## Available Templates

### 🐛 Bug Report (`bug_report.yml`)
Use this template to report bugs, errors, or unexpected behavior in Wassette.

**Includes:**
- Description of the bug
- Steps to reproduce
- Expected vs. actual behavior
- Error logs
- Version and system information
- Component details

**Automatically adds label:** `bug`

### ✨ Feature Request (`feature_request.yml`)
Use this template to propose new features or enhancements.

**Includes:**
- Feature summary and motivation
- Proposed solution
- Feature area classification
- Alternative approaches
- Usage examples
- Breaking change assessment

**Automatically adds label:** `enhancement`

### 📚 Documentation Improvement (`documentation.yml`)
Use this template to suggest improvements to documentation or report documentation issues.

**Includes:**
- Documentation type and location
- Issue type (missing, incorrect, unclear, etc.)
- Current content
- Suggested improvements

**Automatically adds label:** `documentation`

### ❓ Question / Help (`question.yml`)
Use this template to ask questions or get help using Wassette.

**Includes:**
- Your question
- Topic area
- What you've tried
- Context and setup
- Relevant code or configuration

**Automatically adds label:** `question`

**Note:** For general discussions, consider using [GitHub Discussions](https://github.com/microsoft/wassette/discussions) instead.

### 🔒 Security Issue (`security.yml`)
Use this template for non-sensitive security improvements or discussions.

**Important:** For sensitive security vulnerabilities, use [GitHub's private vulnerability reporting](https://github.com/microsoft/wassette/security/advisories/new) instead of filing a public issue.

**Includes:**
- Severity assessment
- Security category
- Potential impact
- Affected versions
- Suggested mitigation

**Automatically adds label:** `security`

## Template Configuration (`config.yml`)

The `config.yml` file configures the issue template chooser and provides helpful links:

- **Blank Issues:** Enabled (allows creating issues without templates when needed)
- **Contact Links:**
  - GitHub Discussions for community questions
  - Documentation site
  - Private security reporting
  - Code of Conduct

## Best Practices

### For Issue Reporters

1. **Choose the right template** - Select the template that best matches your issue type
2. **Fill out all required fields** - These are marked with an asterisk (*)
3. **Be specific** - Provide detailed information to help maintainers understand your issue
4. **Search first** - Check if a similar issue already exists before creating a new one
5. **Follow up** - Respond to questions from maintainers to help resolve your issue

### For Maintainers

1. **Template Updates** - Keep templates aligned with evolving project needs
2. **Label Management** - Ensure labels specified in templates exist in the repository
3. **Triage Process** - Use template fields to quickly understand and categorize issues
4. **Documentation** - Keep CONTRIBUTING.md and other docs in sync with templates

## Template Maintenance

### Adding a New Template

1. Create a new `.yml` file in this directory
2. Follow the GitHub issue form schema (see [documentation](https://docs.github.com/en/communities/using-templates-to-encourage-useful-issues-and-pull-requests/syntax-for-issue-forms))
3. Test the template by creating a test issue
4. Update this README
5. Update CONTRIBUTING.md if needed

### Modifying Existing Templates

1. Edit the `.yml` file
2. Validate YAML syntax: `python3 -c "import yaml; yaml.safe_load(open('template.yml'))"`
3. Test changes by creating a test issue
4. Update this README if the changes are significant

### Common Fields

Most templates include these common elements:
- `name` - Template name shown in the issue chooser
- `description` - Brief description of when to use this template
- `title` - Default issue title prefix
- `labels` - Auto-applied labels
- `body` - Array of form fields (markdown, textarea, input, dropdown, checkboxes)

## YAML Validation

To validate all templates:

```bash
for file in .github/ISSUE_TEMPLATE/*.yml; do 
    python3 -c "import yaml; yaml.safe_load(open('$file'))" && echo "✓ $file" || echo "✗ $file"
done
```

## Resources

- [GitHub Issue Forms Documentation](https://docs.github.com/en/communities/using-templates-to-encourage-useful-issues-and-pull-requests/configuring-issue-templates-for-your-repository)
- [Issue Form Schema Reference](https://docs.github.com/en/communities/using-templates-to-encourage-useful-issues-and-pull-requests/syntax-for-issue-forms)
- [Wassette Contributing Guidelines](../../CONTRIBUTING.md)
