#!/bin/bash
# Setup script for AUR repository

# Clone the AUR package repository (run after creating on AUR website)
git clone ssh://aur@aur.archlinux.org/script-lang.git

cd script-lang

# Copy PKGBUILD from main repo
cp ../../pkg/arch/PKGBUILD .

# Generate .SRCINFO
makepkg --printsrcinfo > .SRCINFO

# Initial commit
git add PKGBUILD .SRCINFO
git commit -m "Initial upload: script-lang 0.5.0alpha"

echo "Ready to push! Run: git push origin master"