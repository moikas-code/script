#!/bin/bash
# Script to update AUR package

if [ $# -eq 0 ]; then
    echo "Usage: $0 <new_version>"
    echo "Example: $0 0.5.1alpha"
    exit 1
fi

NEW_VERSION=$1
PKGNAME="script-lang"

# Navigate to AUR directory
cd "$(dirname "$0")/$PKGNAME" || exit 1

# Update PKGBUILD version
sed -i "s/pkgver=.*/pkgver=$NEW_VERSION/" PKGBUILD

# Reset pkgrel to 1 for new version
sed -i "s/pkgrel=.*/pkgrel=1/" PKGBUILD

# Generate new .SRCINFO
makepkg --printsrcinfo > .SRCINFO

# Commit changes
git add PKGBUILD .SRCINFO
git commit -m "Update to version $NEW_VERSION"

echo "Ready to push! Run: git push origin master"
echo "Don't forget to create a git tag v$NEW_VERSION in your main repo first!"