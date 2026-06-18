# Contributing to Nozich Open Source Projects

Thank you for your interest in contributing! To maintain enterprise-grade software standards, we follow a structured contribution flow.

## 1. Branching Strategy & Development Workflow

We follow a strict git branching strategy:
- **`main` / `master`**: Stable release branch. Direct pushes are disabled/protected.
- **`feat/*`**: For new features and enhancements.
- **`fix/*`**: For bug fixes.
- **`refactor/*`**: For code refactoring or cleaning.
- **`docs/*`**: For documentation changes.

### Workflow:
1. Fork the repository.
2. Create a feature branch from `main` (e.g., `feat/add-json-parser`).
3. Write clean, modular, and documented code.
4. Add unit and integration tests to verify your changes.
5. Verify linting and formatting locally.
6. Commit your changes (must be GPG/SSH signed).
7. Push to your fork and submit a Pull Request.

## 2. Commit Message Guidelines

We enforce **Conventional Commits**:
- Format: `<type>(<scope>): <description>`
- Examples:
  - `feat(gateway): add multi-tenant routing rule matcher`
  - `fix(cli): resolve command-line parser crash on null value`
  - `docs(readme): update installation binary links`

## 3. Commit Verification (GPG/SSH Signing)

To verify contributor identity, **all commits must be signed**. Ensure your git client is configured with a valid signing key:
```bash
git config --global commit.gpgsign true
```
Unsigned pull requests will fail validation checks.

## 4. Coding Standards

- **Rust**: Format code using `cargo fmt` and ensure no warnings are returned by `cargo clippy`.
- **TypeScript / Node**: Follow standard ESLint rules and compile with strict type checking.
