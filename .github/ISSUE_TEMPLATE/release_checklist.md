---
name: Release Checklist
about: Checklist for preparing a new Script release
title: '[RELEASE] v0.0.0'
labels: release
assignees: moikapy

---

## Release Checklist for vX.X.X

### Pre-release
- [ ] All tests passing on main branch
- [ ] No critical security advisories
- [ ] CHANGELOG.md updated with all changes
- [ ] Version bumped in Cargo.toml
- [ ] README.md updated if needed
- [ ] Documentation updated
- [ ] KB files updated and organized

### Testing
- [ ] Full test suite passes locally
- [ ] Benchmarks run without regression
- [ ] Manual testing of key features
- [ ] Examples all run correctly
- [ ] REPL tested interactively

### Cross-platform verification
- [ ] Linux build tested
- [ ] macOS build tested  
- [ ] Windows build tested
- [ ] Auto-updater tested

### Release
- [ ] Create and push version tag
- [ ] Wait for GitHub Actions to build releases
- [ ] Verify all artifacts uploaded
- [ ] Update release notes on GitHub
- [ ] Test download and installation

### Post-release
- [ ] Announcement prepared
- [ ] Update homebrew formula (if applicable)
- [ ] Update package managers
- [ ] Social media announcement
- [ ] Update website/docs (if applicable)

### Verification
- [ ] Auto-updater successfully updates from previous version
- [ ] Clean installation works on all platforms
- [ ] No regression in benchmarks

## Notes
<!-- Add any specific notes about this release -->

## Breaking Changes
<!-- List any breaking changes -->

## Migration Guide
<!-- If there are breaking changes, provide migration instructions -->