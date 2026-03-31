---
name: kvm-dev-workflow
description: Use when developing KVMini cross-platform mouse and keyboard sharing software, including version management, testing, refactoring, and deployment workflows
---

# KVMini Development Workflow

## Overview

Comprehensive development workflow for KVMini cross-platform mouse and keyboard sharing software. This skill enforces quality standards through automated testing, version management, and systematic development practices.

## Core Principles

- **Test-First Development**: Write tests before implementation
- **Version Control**: Automated version management with semantic versioning
- **Quality Assurance**: Continuous refactoring and performance optimization
- **Automation**: Minimize manual intervention in development workflow

## Development Workflow

### 1. Version Management

**Before building any release:**
- Read current version from `package.json` and `src-tauri/Cargo.toml`
- Increment version number by 1
- Update version in all configuration files
- Commit version changes with message: `chore: bump version to vX.Y.Z`

### 2. Test-Driven Development

**Before modifying any code:**
- Analyze requirements and changes
- Write comprehensive test cases covering all scenarios
- Run tests automatically without user intervention
- Ensure all tests pass before proceeding

### 3. Code Quality Assurance

**Before each commit:**
- Refactor code for simplicity and efficiency
- Remove redundant code and optimize performance
- Run complete test suite to ensure functionality
- Build and test Release version
- Generate comprehensive summary report

### 4. Commit Standards

**Commit message format:**
```
[当前版本号] 需求清单总结

- 功能1: 描述
- 功能2: 描述
- 修复: 问题描述
```

### 5. Error Handling Protocol

**When errors occur:**
- Create new test case to reproduce the error
- Ensure error scenario is covered in test suite
- Document error and resolution in commit history

### 6. Problem Resolution Strategy

**For issues requiring 3+ modifications:**
- Alert developer to search for similar engineering solutions
- Research root cause before implementing fixes
- Document findings and approach in development log

### 7. Skill Integration

**Required skill:** `using-superpowers`
- Always use Superpower skill for development tasks
- Install if not available before proceeding

### 8. Automated Deployment

**After successful testing:**
- Automatically start application without waiting
- Verify application functionality in runtime
- Provide deployment confirmation

### 9. Branch Merge Protocol

**During branch merges with version conflicts:**
- Analyze version increments in conflicting branch
- Apply same version increments to current branch
- Resolve conflicts with semantic versioning principles

## Quick Reference

| Task | Command | Purpose |
|------|---------|---------|
| Version Update | `npm run version:update` | Increment version numbers |
| Test Suite | `npm test` | Run all test cases |
| Build Release | `npm run build` | Create optimized release build |
| Development | `npm run dev` | Start development server |

## File Structure Reference

```
KVMini/
├── package.json          # Frontend version and scripts
├── src-tauri/
│   └── Cargo.toml        # Rust backend version
├── __tests__/            # Test cases directory
├── scripts/
│   └── version-update.js # Version management script
└── src/                  # Source code
```

## Common Scenarios

### Adding New Feature
1. Write test cases for new functionality
2. Implement feature with TDD approach
3. Refactor and optimize code
4. Run complete test suite
5. Update version and commit
6. Build release version
7. Start application for verification

### Fixing Bugs
1. Create test case reproducing the bug
2. Implement fix with test validation
3. Add regression test for the bug
4. Follow standard commit workflow

### Performance Optimization
1. Identify performance bottlenecks
2. Write performance test cases
3. Implement optimizations
4. Verify performance improvements
5. Update documentation

## Quality Metrics

- **Test Coverage**: Minimum 80% coverage required
- **Code Quality**: No linting errors or warnings
- **Performance**: Optimized release builds
- **Documentation**: Clear commit messages and summaries

## Error Prevention

- Always run tests before and after changes
- Use version control for all modifications
- Document all changes and decisions
- Follow semantic versioning principles
- Maintain backward compatibility when possible

## Success Verification

- All tests pass without manual intervention
- Release build compiles successfully
- Application starts and functions correctly
- Version numbers are properly incremented
- Commit history is clean and descriptive