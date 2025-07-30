#!/bin/bash

# PCSC Tester Release Helper Script
# Usage: ./release.sh <version> [--prerelease]

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
print_status() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check if version is provided
if [ -z "$1" ]; then
    print_error "Version is required!"
    echo "Usage: $0 <version> [--prerelease]"
    echo "Example: $0 v1.0.0"
    echo "Example: $0 v1.0.0-beta.1 --prerelease"
    exit 1
fi

VERSION="$1"
PRERELEASE="false"

# Check if prerelease flag is set
if [ "$2" = "--prerelease" ]; then
    PRERELEASE="true"
    print_warning "This will be marked as a pre-release"
fi

# Validate version format
if [[ ! $VERSION =~ ^v[0-9]+\.[0-9]+\.[0-9]+.*$ ]]; then
    print_error "Invalid version format. Expected: vX.Y.Z (e.g., v1.0.0)"
    exit 1
fi

# Extract numeric version (without 'v' prefix)
NUMERIC_VERSION="${VERSION#v}"

print_status "Preparing release $VERSION..."

# Check if we're on main branch
CURRENT_BRANCH=$(git rev-parse --abbrev-ref HEAD)
if [ "$CURRENT_BRANCH" != "main" ]; then
    print_warning "You're not on the main branch (current: $CURRENT_BRANCH)"
    read -p "Continue anyway? (y/N): " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        print_error "Aborted"
        exit 1
    fi
fi

# Check if working directory is clean
if [ -n "$(git status --porcelain)" ]; then
    print_error "Working directory is not clean. Please commit or stash your changes."
    git status --short
    exit 1
fi

# Check if tag already exists
if git rev-parse "$VERSION" >/dev/null 2>&1; then
    print_error "Tag $VERSION already exists!"
    exit 1
fi

# Update version in Cargo.toml
print_status "Updating version in Cargo.toml..."
if [[ "$OSTYPE" == "darwin"* ]]; then
    # macOS
    sed -i '' "s/^version = \".*\"/version = \"$NUMERIC_VERSION\"/" Cargo.toml
else
    # Linux
    sed -i "s/^version = \".*\"/version = \"$NUMERIC_VERSION\"/" Cargo.toml
fi

# Update Cargo.lock
print_status "Updating Cargo.lock..."
cargo check --quiet

# Check if there are changes to commit
if [ -n "$(git status --porcelain)" ]; then
    print_status "Committing version changes..."
    git add Cargo.toml Cargo.lock
    git commit -m "chore: bump version to $VERSION

🤖 Generated with [Claude Code](https://claude.ai/code)

Co-Authored-By: Claude <noreply@anthropic.com>"
    print_success "Version bump committed"
else
    print_status "No changes to commit"
fi

# Build and test
print_status "Running tests..."
if cargo test --verbose; then
    print_success "All tests passed!"
else
    print_error "Tests failed! Please fix them before releasing."
    exit 1
fi

print_status "Building release binary..."
if cargo build --release; then
    print_success "Release build successful!"
else
    print_error "Release build failed!"
    exit 1
fi

# Test the binary
print_status "Testing release binary..."
if ./target/release/pcsc-tester --version && ./target/release/pcsc-tester --help > /dev/null; then
    print_success "Binary test successful!"
else
    print_error "Binary test failed!"
    exit 1
fi

# Push changes
print_status "Pushing changes to repository..."
git push origin main

print_success "Version $VERSION prepared successfully!"
print_status "Now triggering GitHub release workflow..."

# Check if gh CLI is available
if command -v gh &> /dev/null; then
    print_status "Using GitHub CLI to trigger release..."
    if [ "$PRERELEASE" = "true" ]; then
        gh workflow run release.yml -f version="$VERSION" -f prerelease=true
    else
        gh workflow run release.yml -f version="$VERSION" -f prerelease=false
    fi
    print_success "Release workflow triggered!"
    print_status "Check the progress at: https://github.com/$(gh repo view --json owner,name -q '.owner.login + "/" + .name')/actions"
else
    print_warning "GitHub CLI not found. Please manually trigger the release workflow:"
    echo ""
    echo "1. Go to: https://github.com/YOUR_USERNAME/pcsc-tester/actions/workflows/release.yml"
    echo "2. Click 'Run workflow'"
    echo "3. Enter version: $VERSION"
    if [ "$PRERELEASE" = "true" ]; then
        echo "4. Check 'Mark as pre-release'"
    fi
    echo "5. Click 'Run workflow'"
fi

print_success "Release process initiated for $VERSION!"