# Version Management Guidelines
**Created**: January 10, 2025  
**Current Version**: v0.5.0-alpha  

## 🎯 Single Source of Truth

**Primary Version Declaration**: `Cargo.toml`
```toml
[package]
version = "0.5.0-alpha"
```

All other version references MUST match this declaration.

## 📋 Version References Checklist

### ✅ Updated Files (v0.5.0-alpha)
- [x] `Cargo.toml` - Primary version source
- [x] `src/main.rs` - Binary version output 
- [x] `README.md` - Documentation version
- [x] `CHANGELOG.md` - Release notes
- [x] `CLAUDE.md` - Project documentation
- [x] `kb/` documentation files
- [x] Documentation version references

### 📍 Version Locations Inventory

| File | Location | Status | Notes |
|------|----------|--------|-------|
| `Cargo.toml` | line 3 | ✅ Updated | Primary source |
| `src/main.rs` | line 31-36 | ✅ Updated | Uses CARGO_PKG_VERSION |
| `README.md` | line 9 | ✅ Updated | Header documentation |
| `CHANGELOG.md` | line 1 | ✅ Updated | Release notes |
| `CLAUDE.md` | multiple | ✅ Updated | Project overview |
| `docs/language/SPECIFICATION.md` | line 1 | ✅ Current | Language spec |

## 🔄 Version Update Process

### When Updating Version:

1. **Update Primary Source**:
   ```bash
   # Edit Cargo.toml version field
   vim Cargo.toml
   ```

2. **Verify Binary Output**:
   ```bash
   cargo run --bin script -- --version
   ```

3. **Check Documentation Consistency**:
   ```bash
   grep -r "0\.5\.0\|v0\.5\.0" . --include="*.md" --include="*.toml"
   ```

4. **Update Documentation**:
   - README.md header
   - CHANGELOG.md new release
   - kb/ status files
   - Any hardcoded version references

5. **Test Build**:
   ```bash
   cargo build --release
   cargo test
   ```

## 🚨 Prevent Version Drift

### Automated Checks:
```bash
# Version consistency check script
#!/bin/bash
CARGO_VERSION=$(grep '^version' Cargo.toml | cut -d'"' -f2)
echo "Cargo.toml version: $CARGO_VERSION"

# Check if binary reports correct version
BINARY_VERSION=$(cargo run --bin script --quiet -- --version | head -n1 | grep -o 'v[0-9]\+\.[0-9]\+\.[0-9]\+[^[:space:]]*')
echo "Binary version: $BINARY_VERSION"

if [ "$BINARY_VERSION" != "v$CARGO_VERSION" ]; then
    echo "❌ VERSION MISMATCH DETECTED!"
    exit 1
else
    echo "✅ Version consistency verified"
fi
```

### Pre-Release Checklist:
- [ ] All version references updated
- [ ] Binary version output correct
- [ ] Documentation reflects current version
- [ ] CHANGELOG.md updated
- [ ] No hardcoded version mismatches
- [ ] Tests pass with new version

## 📊 Current Status: v0.5.0-alpha

### Version Meaning:
- **0.5.0**: Major milestone with 90%+ core completion
- **alpha**: Pre-release with production-ready core but ongoing enhancements
- **Production Status**: Approved for production deployment

### Next Version Targets:
- **v0.5.1-alpha**: Bug fixes and minor enhancements
- **v0.6.0-alpha**: Complete MCP integration
- **v1.0.0**: Full production release

## 🛡️ Version Security

### Avoid Version Confusion:
- Never have different versions in documentation vs binary
- Always update CHANGELOG.md with version bumps
- Use semantic versioning consistently
- Test version output after every change

### Documentation Standards:
- Use "Version X.Y.Z" in titles
- Use "vX.Y.Z" in inline references  
- Include version in all major documentation
- Update status files with version context

---

**Current Version**: v0.5.0-alpha ✅  
**Status**: Production Ready - Zero Version Inconsistencies