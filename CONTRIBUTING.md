# Contributing to Loom

Thank you for your interest in contributing to Loom! This document provides guidelines and workflows for contributing.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [Development Workflow](#development-workflow)
- [Coding Standards](#coding-standards)
- [Commit Message Format](#commit-message-format)
- [Pull Request Process](#pull-request-process)
- [Testing Requirements](#testing-requirements)
- [Documentation Requirements](#documentation-requirements)
- [Code Review](#code-review)
- [Reporting Issues](#reporting-issues)

## Code of Conduct

By participating in this project, you agree to maintain a respectful and inclusive environment. Be kind, constructive, and considerate in all interactions.

## Getting Started

### 1. Fork & Clone

```bash
# Fork the repository on GitHub
# Then clone your fork
git clone https://github.com/YOUR_USERNAME/loom.git
cd loom

# Add upstream remote
git remote add upstream https://github.com/ghuntley/loom.git
```

### 2. Set Up Development Environment

Follow the setup guide for your preferred method:

- **Quick Setup**: See [DEV_ENVIRONMENT_SETUP.md](DEV_ENVIRONMENT_SETUP.md)
- **With Nix**: See [DEV_ENVIRONMENT_NIX.md](DEV_ENVIRONMENT_NIX.md)

```bash
# Verify setup
make check
```

### 3. Create a Branch

```bash
# Update main branch
git fetch upstream
git checkout main
git reset --hard upstream/main

# Create feature branch
git checkout -b feature/your-feature-name
```

## Development Workflow

### Before Writing Code

1. **Check existing issues**: https://github.com/ghuntley/loom/issues
2. **Search discussions**: https://github.com/ghuntley/loom/discussions
3. **Open an issue** if proposing something new
4. **Get approval** for significant changes

### While Coding

```bash
# 1. Make your changes
# Edit files in your editor

# 2. Run tests frequently
make test

# 3. Check code quality
make lint

# 4. Format code
make format

# 5. Run full CI checks
make check

# 6. Commit with proper message (see below)
git add .
git commit -m "type(scope): description"

# 7. Push to your fork
git push origin feature/your-feature-name
```

### Continuous Development

```bash
# Keep your branch updated with upstream
git fetch upstream
git rebase upstream/main

# Or if you have local commits
git rebase upstream/main -i

# Force push only after rebase (use with care)
git push --force-with-lease origin feature/your-feature-name
```

## Coding Standards

### Rust Code

We follow Rust best practices and the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/).

#### Style & Formatting

```bash
# Automatic formatting
cargo fmt --all

# Check formatting
cargo fmt --all -- --check

# Our setup runs this automatically via:
make fmt
make format
```

#### Linting

```bash
# Run clippy with strict warnings
cargo clippy --all -- -D warnings

# Auto-fix some issues
cargo fix --all

# Our setup runs this via:
make lint
make fix
```

#### Guidelines

- **Naming**: Use `snake_case` for variables/functions, `CamelCase` for types
- **Comments**: Document public APIs with doc comments (`///`)
- **Error handling**: Use `Result<T>` and `?` operator, avoid panics in libraries
- **Unsafe code**: Minimize and document with `// SAFETY:` comments
- **Dependencies**: Minimize external dependencies, review security

### Example: Well-formatted Rust

```rust
/// Processes a query and returns the result.
///
/// # Arguments
///
/// * `query` - The query to process
/// * `options` - Configuration options
///
/// # Returns
///
/// Returns a `Result` containing the processed data or an error
///
/// # Errors
///
/// This function will return an error if:
/// - The query is invalid
/// - Processing fails
pub fn process_query(query: &str, options: Options) -> Result<Data, Error> {
    // Implementation
    Ok(Data::default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_query() {
        let result = process_query("test", Options::default());
        assert!(result.is_ok());
    }
}
```

## Commit Message Format

Follow the [Conventional Commits](https://www.conventionalcommits.org/) format:

```
<type>(<scope>): <subject>

<body>

<footer>
```

### Type

Must be one of:
- **feat**: A new feature
- **fix**: A bug fix
- **docs**: Documentation only changes
- **style**: Changes that don't affect code meaning (formatting, etc)
- **refactor**: Code change that neither fixes a bug nor adds a feature
- **perf**: Code change that improves performance
- **test**: Adding or updating tests
- **chore**: Changes to build process, dependencies, or tooling
- **ci**: Changes to CI/CD configuration

### Scope

The scope specifies what area of the code is affected:
- `loom-core` - Core functionality
- `loom-server` - Server implementation
- `loom-cli` - Command-line interface
- `llm` - LLM providers (anthropic, openai, etc)
- Or any other relevant scope

### Subject

- Use imperative mood ("add" not "added" or "adds")
- Don't capitalize first letter
- No period (.) at the end
- Maximum 50 characters

### Body

- Explain **what** and **why**, not how
- Wrap at 72 characters
- Separate from subject with blank line
- Optional but recommended for non-trivial changes

### Footer

Optional. Use for:
- Breaking changes: `BREAKING CHANGE: description`
- Issue references: `Fixes #123`
- Co-authors: `Co-authored-by: Name <email>`

### Examples

```
feat(loom-core): add streaming query support

Implement streaming responses for long-running queries,
allowing clients to receive results incrementally instead
of waiting for complete results.

Fixes #456
```

```
docs(README): update installation instructions
```

```
refactor(loom-server): simplify error handling

Extract common error handling logic into reusable functions
to reduce code duplication across handler functions.
```

## Pull Request Process

### Creating a PR

1. **Push your branch**:
   ```bash
   git push origin feature/your-feature-name
   ```

2. **Create PR on GitHub**:
   - Title: Same as first commit message
   - Description: Explain the changes
   - Link related issues: "Fixes #123"
   - Request reviewers

3. **PR Template** (use the template provided):

```markdown
## Description
Brief description of what this PR does

## Related Issue
Fixes #123

## Type of Change
- [ ] Bug fix
- [ ] New feature
- [ ] Breaking change
- [ ] Documentation update

## Changes Made
- Change 1
- Change 2

## Testing
How was this tested?

## Checklist
- [ ] Tests added/updated
- [ ] Documentation updated
- [ ] Code formatted
- [ ] No linter warnings
```

### PR Guidelines

- **Size**: Keep PRs focused (ideally <400 lines changed)
- **Tests**: Include tests for new features
- **Documentation**: Update docs if behavior changes
- **Commits**: Keep commit history clean (rebase if needed)
- **Draft**: Use draft PR if still in progress

### CI Requirements

All PR checks must pass:
- ✅ Format check (`cargo fmt`)
- ✅ Lint check (`cargo clippy`)
- ✅ Cargo check (`cargo check`)
- ✅ Unit tests (`cargo test`)
- ✅ Integration tests
- ✅ Security audit
- ✅ Build check
- ✅ Documentation check

## Testing Requirements

### Writing Tests

All new features must include tests. We use property-based testing and comprehensive unit tests.

#### Test Structure

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_functionality() {
        // Arrange
        let input = "test";
        
        // Act
        let result = process(input);
        
        // Assert
        assert_eq!(result, "expected");
    }

    #[test]
    fn test_error_case() {
        let result = process("");
        assert!(result.is_err());
    }
}
```

#### Property-Based Tests

For complex logic, use property-based tests:

```rust
#[cfg(test)]
mod prop_tests {
    use proptest::prelude::*;
    use super::*;

    proptest! {
        /// Test that parsing and serializing produces the same data.
        /// This is important because it ensures round-trip compatibility.
        #[test]
        fn prop_roundtrip(s in "\\PC*") {
            let parsed = parse(&s)?;
            let serialized = serialize(&parsed)?;
            let reparsed = parse(&serialized)?;
            prop_assert_eq!(parsed, reparsed);
        }
    }
}
```

### Running Tests

```bash
# Run all tests
make test

# Run specific test
cargo test -p loom-core test_name

# Run tests with output
cargo test -- --nocapture

# Run integration tests only
cargo test --test '*'

# Run with logging
RUST_LOG=debug cargo test -- --nocapture
```

### Test Coverage

Aim for >80% code coverage on new code. Check with:

```bash
# Requires cargo-tarpaulin
cargo install cargo-tarpaulin
cargo tarpaulin --out Html
```

## Documentation Requirements

### Code Documentation

- **Public items** must have doc comments (`///`)
- **Module-level** documentation at top of file
- **Examples** in doc comments where helpful
- **Errors** documented in `// Errors` section

### Example: Good Documentation

```rust
/// Processes a vector of queries in parallel.
///
/// This function spawns a task for each query and collects results.
///
/// # Arguments
///
/// * `queries` - Slice of queries to process
/// * `max_concurrent` - Maximum concurrent operations
///
/// # Returns
///
/// A vector of results, preserving input order
///
/// # Errors
///
/// Returns an error if any query processing fails
///
/// # Examples
///
/// ```
/// use loom_core::process_queries;
///
/// # tokio::runtime::Runtime::new().unwrap().block_on(async {
/// let queries = vec!["query1", "query2"];
/// let results = process_queries(&queries, 4).await?;
/// assert_eq!(results.len(), 2);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// # })
/// ```
pub async fn process_queries(
    queries: &[&str],
    max_concurrent: usize,
) -> Result<Vec<Data>, Error> {
    // Implementation
    Ok(vec![])
}
```

### User Documentation

For user-facing changes:
- Update relevant `.md` files in root
- Add examples if introducing new features
- Update API documentation
- Update CLI help text

## Code Review

### Review Checklist

Reviewers will check:

- [ ] Code is clear and well-documented
- [ ] Tests cover new functionality
- [ ] No unnecessary dependencies added
- [ ] Performance not degraded
- [ ] Error handling is appropriate
- [ ] Security implications considered
- [ ] Follows project style guide

### Responding to Reviews

- Be respectful and collaborative
- Ask clarifying questions
- Make requested changes promptly
- Push updates as new commits (don't amend)
- Mark conversations as resolved when done

### After Approval

- Ensure all CI checks pass
- Squash commits if requested: `git rebase -i main`
- Ensure branch is up to date
- Maintainer will merge when ready

## Reporting Issues

### Before Reporting

1. Check if issue already exists
2. Search closed issues
3. Check documentation and FAQs
4. Try latest development version

### Creating an Issue

**Title**: Clear, concise description
**Labels**: Add relevant labels
**Template**: Use the issue template provided

### Bug Report

```markdown
## Description
What is the bug?

## Reproduction Steps
1. Step 1
2. Step 2

## Expected Behavior
What should happen?

## Actual Behavior
What actually happened?

## Environment
- OS: ...
- Rust version: ...
- Loom version: ...

## Logs/Output
Include relevant logs with backtrace if available

## Possible Solution
(Optional) Ideas for fixing the bug
```

### Feature Request

```markdown
## Description
What feature would you like?

## Motivation
Why is this needed?

## Examples
How would this be used?

## Alternatives
Have you considered other approaches?
```

## Tips for Successful Contributions

1. **Start small**: Fix typos, improve docs before major features
2. **Communicate**: Discuss big changes in issues first
3. **Read code**: Understand existing patterns before writing new code
4. **Ask questions**: Better to ask than assume
5. **Be patient**: Reviews take time, be responsive
6. **Have fun**: Contributing should be enjoyable!

## Need Help?

- **GitHub Issues**: Ask questions in issue threads
- **Discussions**: Use GitHub discussions for ideas
- **Documentation**: Check `.md` files for answers
- **Community**: Join our community channels

## License

By contributing to Loom, you agree that your contributions will be licensed under the MIT License.

---

Thank you for contributing to Loom! We appreciate your effort and dedication.
