# Development Workflow

Branching strategy, commit conventions, and development processes.

## Branching Strategy

### Main Branches

```
main (protected)
  │
  ├── develop
  │     │
  │     ├── feature/agent-metrics
  │     ├── feature/anomaly-detection
  │     └── feature/slack-integration
  │
  ├── hotfix/critical-bug
  └── release/v1.2.0
```

### Branch Types

| Branch Type | Prefix | Purpose | Lifetime |
|-------------|--------|---------|----------|
| Main | `main` | Production code | Permanent |
| Development | `develop` | Integration branch | Permanent |
| Feature | `feature/` | New features | Temporary |
| Bugfix | `bugfix/` | Bug fixes | Temporary |
| Hotfix | `hotfix/` | Critical production fixes | Temporary |
| Release | `release/v*` | Release preparation | Temporary |
| Experiment | `exp/` | Experimental features | Temporary |

### Branch Protection Rules

**main branch:**
- [x] Require pull request reviews
  - Required approving reviews: 2
  - Dismiss stale reviews
- [x] Require status checks
  - All tests must pass
  - Coverage >= 80%
  - No clippy warnings
  - Security audit clean
- [x] Require branches to be up to date
- [x] Block force pushes
- [x] Require linear history

**develop branch:**
- [x] Require pull request reviews
  - Required approving reviews: 1
- [x] Require status checks to pass
- [x] Block force pushes

## Commit Conventions

### Conventional Commits

```
<type>[optional scope]: <description>

[optional body]

[optional footer(s)]
```

### Types

| Type | Purpose | Example |
|------|---------|---------|
| `feat` | New feature | `feat(agent): add Prometheus scrape support` |
| `fix` | Bug fix | `fix(correlation): prevent duplicate alerts` |
| `docs` | Documentation | `docs(api): update authentication examples` |
| `style` | Code style | `style: format code with rustfmt` |
| `refactor` | Refactoring | `refactor(pipeline): extract collector trait` |
| `perf` | Performance | `perf(anomaly): optimize matrix operations` |
| `test` | Tests | `test(agent): add integration tests` |
| `chore` | Build/config | `chore: update dependencies` |
| `ci` | CI/CD | `ci: add benchmark workflow` |
| `build` | Build system | `build: upgrade to Rust 1.85` |
| `revert` | Revert commit | `revert: feat(agent) add cloudwatch support` |

### Scopes

Common scopes:
- `agent` - Telemetry collection agent
- `pipeline` - Data processing pipeline
- `anomaly` - ML anomaly detection
- `correlation` - Alert correlation
- `remediation` - Remediation workflows
- `topology` - Service topology
- `api` - API gateway
- `web` - Web dashboard
- `deploy` - Deployment configs
- `ci` - CI/CD
- `docs` - Documentation

### Examples

```bash
# Simple feature
git commit -m "feat(agent): add CloudWatch metrics collection"

# Bug fix with body
git commit -m "fix(correlation): handle missing labels gracefully

Previously, the correlation engine would panic when encountering
metrics with missing label sets. This commit adds proper error
handling and default label values.

Fixes #123"

# Breaking change
git commit -m "feat(api)!: change alert response format

BREAKING CHANGE: The alert response format has changed to include
additional metadata. Clients will need to update their parsing
logic."

# Feature with multiple commits
git commit -m "feat(anomaly): add LSTM model support"
git commit -m "test(anomaly): add LSTM tests"
git commit -m "docs(anomaly): document LSTM model"
```

### Commit Message Linting

```yaml
# .github/workflows/commit-lint.yml
name: Commit Lint

on:
  pull_request:
    types: [opened, synchronize, reopened]

jobs:
  lint:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0

      - name: Install commitlint
        run: npm install -g @commitlint/cli @commitlint/config-conventional

      - name: Lint commits
        run: |
          commitlint --from $(git merge-base ${{ github.base_ref }} HEAD) \
                     --to HEAD \
                     --verbose
```

## Pull Request Process

### PR Template

```markdown
# Description
<!-- Briefly describe the changes in this PR -->

## Type of Change
- [ ] Bug fix
- [ ] New feature
- [ ] Breaking change
- [ ] Documentation update
- [ ] Performance improvement
- [ ] Code refactoring
- [ ] Other (please describe)

## Related Issue
Fixes #XXX
Related to #YYY

## Changes Made
<!-- List the main changes -->

-
-
-

## Testing
<!-- Describe how you tested these changes -->

-
-

## Checklist
- [ ] My code follows the style guidelines
- [ ] I have performed a self-review
- [ ] I have commented my code, particularly in hard-to-understand areas
- [ ] I have made corresponding changes to the documentation
- [ ] My changes generate no new warnings
- [ ] I have added tests that prove my fix is effective or that my feature works
- [ ] New and existing tests pass locally
- [ ] I have updated the changelog
- [ ] My changes are ready for review

## Screenshots (if applicable)
<!-- Add screenshots to help explain your changes -->

## Additional Notes
<!-- Any additional information -->
```

### PR Review Process

1. **Create PR**
   - Fill out template completely
   - Link related issues
   - Request 2 reviewers
   - Apply appropriate labels

2. **Automated Checks**
   - CI must pass
   - Coverage >= 80%
   - No security vulnerabilities
   - Benchmarks stable

3. **Review**
   - At least 2 approvals required
   - Address all review comments
   - Update PR as needed
   - Re-request review after changes

4. **Merge**
   - Squash and merge to maintain clean history
   - Delete branch after merge
   - Update related issues
   - Notify team in Slack

### Review Guidelines

**Reviewers:**
- Respond to reviews within 24 hours
- Be constructive and respectful
- Explain the "why" for changes
- Approve when satisfied or offer alternatives
- Test locally if needed

**Authors:**
- Be open to feedback
- Explain your reasoning
- Make requested changes promptly
- Ask for clarification if needed
- Don't take criticism personally

## Development Workflow

### 1. Start New Feature

```bash
# Update main and develop
git checkout main
git pull

# Create feature branch from develop
git checkout develop
git pull
git checkout -b feature/my-feature

# Set up tracking
git push -u origin feature/my-feature
```

### 2. Development Cycle

```bash
# Watch mode for development
make watch

# In another terminal, run tests
make test-watch

# Make changes
# ...

# Check formatting
make fmt

# Run linter
make lint

# Run tests
make test

# Run integration tests
make test-integration

# Check coverage
make test-coverage
```

### 3. Before Submitting PR

```bash
# Ensure everything passes
make ci

# Update changelog
vim CHANGELOG.md

# Commit all changes
git add .
git commit -m "feat: my feature"

# Push to remote
git push
```

### 4. Create Pull Request

```bash
# Using GitHub CLI
gh pr create \
  --title "feat: add my feature" \
  --body "Description of changes..." \
  --base develop \
  --reviewer @rustops/maintainers
```

### 5. Address Review Feedback

```bash
# Make changes
# ...

# Commit fixes
git add .
git commit -m "fix: address review feedback"

# Push updates
git push

# Re-request review
gh pr edit --add-reviewer @reviewer
```

### 6. Merge PR

```bash
# After approval and CI passes
gh pr merge --squash --delete-branch

# Update local develop
git checkout develop
git pull

# Delete local branch
git branch -d feature/my-feature
```

## Hotfix Process

### For Critical Production Issues

```bash
# Create hotfix branch from main
git checkout main
git pull
git checkout -b hotfix/critical-bug

# Make fix
# ...

# Test thoroughly
make test
make test-integration

# Commit
git add .
git commit -m "fix: critical bug in production"

# Push and create PR to main
git push -u origin hotfix/critical-bug
gh pr create --base main --title "fix: critical bug"

# Merge with approval
gh pr merge --squash

# Create release tag
git tag -a v1.2.1 -m "Release v1.2.1"
git push origin v1.2.1

# Merge back to develop
git checkout develop
git merge hotfix/critical-bug
git push origin develop

# Delete hotfix branch
git branch -d hotfix/critical-bug
```

## Release Process

### 1. Create Release Branch

```bash
# From develop
git checkout develop
git pull
git checkout -b release/v1.2.0

# Update version
vim Cargo.toml  # Change version to 1.2.0

# Update CHANGELOG
vim CHANGELOG.md
```

### 2. Finalize Release

```bash
# Run full test suite
make ci

# Build release binaries
make build-release

# Test locally
# ...

# Commit version and changelog
git add .
git commit -m "chore: release v1.2.0"

# Push to remote
git push -u origin release/v1.2.0
```

### 3. Create PR to Main

```bash
gh pr create \
  --base main \
  --title "Release v1.2.0" \
  --body "Release notes:..."
```

### 4. Merge and Tag

```bash
# After approval
gh pr merge --squash

# Create and push tag
git checkout main
git pull
git tag -a v1.2.0 -m "Release v1.2.0"
git push origin v1.2.0

# GitHub Actions will:
# - Build release binaries
# - Create GitHub release
# - Build and push containers
# - Deploy to staging
```

### 5. Merge Back to Develop

```bash
git checkout develop
git merge main
git push origin develop
```

## Code Review Standards

### What to Look For

**Functionality:**
- Does it work as intended?
- Are edge cases handled?
- Is error handling comprehensive?

**Code Quality:**
- Is it readable and maintainable?
- Does it follow Rust best practices?
- Are there any code smells?

**Testing:**
- Are there sufficient tests?
- Do tests cover edge cases?
- Are tests well-written?

**Documentation:**
- Is the code documented?
- Are examples clear?
- Is the API intuitive?

**Performance:**
- Is it performant?
- Are there obvious bottlenecks?
- Could it be optimized?

**Security:**
- Are there security concerns?
- Is input validated?
- Are secrets handled properly?

### Review Response Guide

**Approval:**
```
LGTM! Great work on this.

Minor nit: line 42 could use a comment explaining the algorithm.
```

**Request Changes:**
```
This looks good overall, but I have a few concerns:

1. Error handling on line 25 could be more specific
2. Test coverage is below 80%
3. Documentation is missing for the public API

Could you address these before I approve?
```

**Comments:**
```
Nice approach! I hadn't considered doing it this way.

One suggestion: you could simplify line 78 using itertools.
```

## Team Communication

### Slack Notifications

**PR created:**
```
@here New PR: "feat: add anomaly detection"
https://github.com/rustops/rustops/pull/123

Author: @username
Reviewers: @reviewer1 @reviewer2
```

**PR merged:**
```
✅ Merged: "feat: add anomaly detection"
https://github.com/rustops/rustops/pull/123

Deploying to dev...
```

**Release deployed:**
```
🚀 Deployed: v1.2.0 to production

Changes:
- Added anomaly detection
- Fixed correlation bug
- Updated dependencies

Monitoring: https://metrics.rustops.dev
```

### Standup Updates

Daily standup format:
```
Yesterday: [What you did]
Today: [What you're doing]
Blockers: [What's blocking you]
PRs: [Open PRs needing review]
Reviews: [PRs you're reviewing]
```

## Workflow Diagram

```
┌──────────────┐
│  Pick Issue  │
└──────┬───────┘
       │
       ▼
┌──────────────┐
│ Create Branch│
│ (feature/*)  │
└──────┬───────┘
       │
       ▼
┌──────────────┐
│  Development │
│              │◄───────┐
│ - Coding     │        │
│ - Testing    │        │ Feedback
│ - Docs       │        │
└──────┬───────┘        │
       │                │
       ▼                │
┌──────────────┐        │
│  Run CI Loc  │        │
│  (make ci)   │        │
└──────┬───────┘        │
       │                │
       ▼                │
┌──────────────┐        │
│   Push & PR  │        │
└──────┬───────┘        │
       │                │
       ▼                │
┌──────────────┐        │
│ Automated CI │        │
│              │        │
│ - Build      │        │
│ - Test       │        │
│ - Lint       │        │
│ - Audit      │        │
└──────┬───────┘        │
       │                │
       ▼                │
┌──────────────┐        │
│   Review     │        │
│   (2+ LGTM)  │────────┘
└──────┬───────┘
       │
       ▼
┌──────────────┐
│   Merge      │
│ (Squash)     │
└──────┬───────┘
       │
       ▼
┌──────────────┐
│  Delete BR   │
└──────────────┘
```

## Best Practices

### DO:
- Write meaningful commit messages
- Keep PRs focused and small
- Update documentation with code changes
- Test thoroughly before submitting
- Respond to reviews promptly
- Ask for help when stuck
- Follow the team coding standards

### DON'T:
- Commit directly to main/develop
- Mix unrelated changes in one PR
- Ignore review feedback
- Skip testing
- Submit huge PRs (>1000 files)
- Block main/develop
- Forget to update CHANGELOG

## Escalation Path

1. **Disagreement on implementation**
   - Discuss in PR comments
   - Schedule a sync call if needed
   - Escalate to tech lead for decision

2. **Blocking issue**
   - Post in #dev channel with details
   - Tag tech lead
   - Create incident ticket if production

3. **Production emergency**
   - Use hotfix process
   - Notify in #incidents channel
   - Postmortem after resolution

## Metrics

Track these development metrics:

| Metric | Target | Current |
|--------|--------|---------|
| PR size (files) | < 50 | - |
| PR review time | < 24h | - |
| Time to merge | < 48h | - |
| Failed builds | < 5% | - |
| Rollback rate | < 1% | - |
| Test coverage | 80%+ | - |
