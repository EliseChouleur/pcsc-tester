# GitHub Workflows Documentation

This directory contains the GitHub Actions workflows for the PCSC Tester project.

## Workflows

### 1. CI Workflow (`ci.yml`)
**Trigger:** Push to `main`/`develop` branches, Pull Requests to `main`

**Purpose:** Automated testing and quality checks

**Jobs:**
- **Test Suite**: Cross-platform testing (Ubuntu, Windows, macOS)
- **Code Coverage**: Generate coverage reports with `cargo-tarpaulin`
- **Security Audit**: Run `cargo audit` for security vulnerabilities
- **Build Check**: Verify release builds work correctly

**Features:**
- Runs on multiple platforms simultaneously
- Caches Cargo registry and build artifacts
- Tests requiring hardware are marked with `#[ignore]` and run separately
- Automated code formatting and linting checks

### 2. Release Workflow (`release.yml`)
**Trigger:** Manual workflow dispatch

**Purpose:** Create GitHub releases with cross-platform binaries

**Process:**
1. Create GitHub release with detailed description
2. Build binaries for all supported platforms:
   - Linux x86_64
   - Windows x86_64
   - macOS x86_64 (Intel)
   - macOS aarch64 (Apple Silicon)
3. Generate SHA256 checksums for all binaries
4. Upload binaries and checksums as release assets
5. Update Cargo.toml version and commit changes

**Usage:**
```bash
# Using GitHub CLI
gh workflow run release.yml -f version="v1.0.0" -f prerelease=false

# Or use the helper script
./.github/release.sh v1.0.0
```

### 3. Auto Release Workflow (`auto-release.yml`)
**Trigger:** Push to `main` with changes to `Cargo.toml`

**Purpose:** Automatically trigger releases when version is bumped

**Process:**
1. Detect version changes in `Cargo.toml`
2. Automatically trigger the release workflow
3. Create release with the new version

### 4. PR Validation Workflow (`pr-validation.yml`)
**Trigger:** Pull request events (opened, synchronized, reopened)

**Purpose:** Validate pull requests before merging

**Checks:**
- Code formatting (`cargo fmt`)
- Linting (`cargo clippy`)
- Tests pass
- Release build works
- PR title follows semantic conventions
- PR size warnings for large changes

## Release Process

### Option 1: Using the Helper Script (Recommended)
```bash
# Navigate to project root
cd pcsc-tester

# Create a new release
./.github/release.sh v1.0.0

# Create a pre-release
./.github/release.sh v1.0.0-beta.1 --prerelease
```

The script will:
- Validate version format
- Check git status
- Update `Cargo.toml` and `Cargo.lock`
- Run tests
- Build release binary
- Commit and push changes
- Trigger GitHub release workflow

### Option 2: Manual Process
1. Update version in `Cargo.toml`
2. Commit and push changes
3. Go to GitHub Actions → Release workflow
4. Click "Run workflow"
5. Enter version (e.g., `v1.0.0`)
6. Choose if it's a pre-release
7. Run workflow

### Option 3: Automatic (Version Bump)
1. Simply update the version in `Cargo.toml`
2. Commit and push to `main`
3. The auto-release workflow will trigger automatically

## Binary Naming Convention

Released binaries follow this pattern:
- `pcsc-tester-{version}-{platform}-{arch}[.exe]`

Examples:
- `pcsc-tester-v1.0.0-linux-x86_64`
- `pcsc-tester-v1.0.0-windows-x86_64.exe`
- `pcsc-tester-v1.0.0-macos-x86_64`
- `pcsc-tester-v1.0.0-macos-aarch64`

## Issue Templates

The `.github/ISSUE_TEMPLATE/` directory contains templates for:
- **Bug Reports**: Structured bug reporting with environment details
- **Feature Requests**: Feature suggestion template with use cases

## Pull Request Template

The `pull_request_template.md` provides a structured format for PRs including:
- Change description
- Type of change
- Testing checklist
- Platform compatibility
- Review checklist

## Development Workflow

1. **Create feature branch**: `git checkout -b feature/my-feature`
2. **Make changes**: Implement your feature/fix
3. **Test locally**: `cargo test && cargo build --release`
4. **Create PR**: Push branch and create pull request
5. **CI validation**: Automatic PR validation runs
6. **Review**: Code review and approval
7. **Merge**: Merge to main branch
8. **Release**: Use release script or manual workflow dispatch

## Environment Variables

The workflows use these environment variables:
- `CARGO_TERM_COLOR=always`: Enable colored Cargo output
- `GITHUB_TOKEN`: Automatically provided by GitHub Actions

## Platform-Specific Notes

### Linux
- Requires `libpcsclite-dev` and `pkg-config`
- Uses system package manager for dependencies

### Windows
- PCSC support is built-in
- No additional system dependencies required

### macOS
- PCSC framework is available by default
- Supports both Intel (x86_64) and Apple Silicon (aarch64)

## Troubleshooting

### Failed Tests
- Check if PCSC daemon is running (for integration tests)
- Hardware-dependent tests are marked with `#[ignore]`
- Review test output for specific failure reasons

### Build Failures
- Ensure all system dependencies are installed
- Check Rust toolchain version compatibility
- Review cross-compilation target availability

### Release Issues
- Verify version format (must start with 'v')
- Check if tag already exists
- Ensure working directory is clean
- Validate GitHub token permissions