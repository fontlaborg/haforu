#!/usr/bin/env bash
# this_file: scripts/sync-version.sh

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo -e "${GREEN}===== Version Sync Script =====${NC}"
echo ""

# Get version from git tag or provide default
if git describe --tags --exact-match 2>/dev/null; then
    VERSION=$(git describe --tags --exact-match | sed 's/^v//')
    echo -e "${GREEN}Found git tag: v$VERSION${NC}"
else
    # Try to get from most recent tag
    if git describe --tags --abbrev=0 2>/dev/null; then
        LAST_TAG=$(git describe --tags --abbrev=0 | sed 's/^v//')
        # Count commits since last tag
        COMMITS_SINCE=$(git rev-list $(git describe --tags --abbrev=0)..HEAD --count)

        if [ "$COMMITS_SINCE" -gt 0 ]; then
            # Development version
            VERSION="${LAST_TAG}.dev${COMMITS_SINCE}"
            echo -e "${YELLOW}No exact tag, using development version: $VERSION${NC}"
        else
            VERSION="$LAST_TAG"
            echo -e "${GREEN}Using most recent tag: v$VERSION${NC}"
        fi
    else
        VERSION="0.0.0.dev"
        echo -e "${YELLOW}No tags found, using default: $VERSION${NC}"
    fi
fi

echo ""
echo "Syncing version: $VERSION"
echo ""

# Update Cargo.toml
if [ -f "Cargo.toml" ]; then
    echo "Updating Cargo.toml..."

    # Create backup
    cp Cargo.toml Cargo.toml.bak

    # Update version line
    if [[ "$OSTYPE" == "darwin"* ]]; then
        # macOS
        sed -i '' "s/^version = \".*\"/version = \"$VERSION\"/" Cargo.toml
    else
        # Linux
        sed -i "s/^version = \".*\"/version = \"$VERSION\"/" Cargo.toml
    fi

    echo -e "${GREEN}✓ Updated Cargo.toml${NC}"
else
    echo -e "${RED}✗ Cargo.toml not found${NC}"
fi

# Check if pyproject.toml uses dynamic versioning
if [ -f "pyproject.toml" ]; then
    if grep -q "dynamic.*version" pyproject.toml; then
        echo "Python version is managed by hatch-vcs (dynamic)"
        echo -e "${GREEN}✓ pyproject.toml uses git-based versioning${NC}"
    else
        echo -e "${YELLOW}Warning: pyproject.toml doesn't use dynamic versioning${NC}"
        echo "Consider adding: dynamic = [\"version\"]"
    fi
fi

echo ""
echo "Version sync complete!"
echo ""
echo "Current versions:"
echo "  Git tag:     $(git describe --tags --abbrev=0 2>/dev/null || echo 'none')"
echo "  Cargo.toml:  $(grep '^version = ' Cargo.toml | head -1 | cut -d'"' -f2)"

if [ -f "python/haforu/__init__.py" ]; then
    if grep -q "__version__" python/haforu/__init__.py 2>/dev/null; then
        echo "  Python:      $(grep '__version__' python/haforu/__init__.py | cut -d'"' -f2 || echo 'dynamic')"
    else
        echo "  Python:      dynamic (from git tags via hatch-vcs)"
    fi
fi

echo ""
echo "To create a new release:"
echo "  1. git tag v$VERSION"
echo "  2. git push --tags"
echo "  3. GitHub Actions will build and publish automatically"
echo ""

# Cleanup backup
if [ -f "Cargo.toml.bak" ]; then
    rm Cargo.toml.bak
fi