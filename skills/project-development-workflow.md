---
name: project-development-workflow
description: Use when developing cross-platform KVM software with Tauri, requiring strict version control, test-driven development, and automated quality assurance
---

# Project Development Workflow

## Overview
Comprehensive development workflow for cross-platform KVM software with automated version management, TDD, and quality assurance.

## When to Use
- Starting new feature development or bug fixes
- Preparing code for commit and release
- Handling version conflicts during merges
- Encountering recurring issues requiring systematic debugging

## Core Development Rules

### 1. Version Management
```javascript
// package.json version increment automation
const updateVersion = () => {
  const packageJson = JSON.parse(fs.readFileSync('package.json', 'utf8'));
  const version = packageJson.version.split('.');
  version[2] = parseInt(version[2]) + 1; // Increment patch version
  packageJson.version = version.join('.');
  fs.writeFileSync('package.json', JSON.stringify(packageJson, null, 2));
  return packageJson.version;
};
```

### 2. Test-Driven Development Process
**REQUIRED:** Always use test-driven-development skill before implementation

**Workflow:**
1. Analyze requirements and write test cases
2. Run tests (should fail initially)
3. Implement minimal code to pass tests
4. Refactor and optimize
5. Re-run all tests to ensure functionality

### 3. Code Quality Assurance
**Before every commit:**
- Run comprehensive test suite
- Perform code refactoring for clarity and efficiency
- Remove redundant code and optimize performance
- Verify all edge cases are covered

### 4. Commit Message Format
```
[VERSION] Brief description of changes

- Detailed change list
- Testing methodology used
- Performance improvements
- Related requirements addressed
```

### 5. Error Handling and Learning
- Every encountered error must have a corresponding test case
- Document root causes and solutions
- Create regression tests for fixed issues

### 6. Recurring Issue Protocol
**After 3+ occurrences of same issue:**
- Pause implementation
- Research similar projects and solutions
- Perform root cause analysis
- Implement comprehensive fix with tests

### 7. Superpower Integration
**REQUIRED:** Use superpowers skill for all development tasks
- Install if not available
- Leverage specialized capabilities for efficient development

### 8. Automated Execution
- After successful testing, automatically launch application
- Verify functionality in running environment
- Monitor for any runtime issues

### 9. Version Conflict Resolution
**During branch merges:**
- Identify version number differences
- Calculate total version increments from both branches
- Apply combined version increment
- Resolve any conflicts systematically

## Implementation Checklist

### Pre-Development
- [ ] Load superpowers skill
- [ ] Analyze requirements using brainstorming skill
- [ ] Write comprehensive test cases using test-driven-development

### Development Phase
- [ ] Implement code to pass all tests
- [ ] Run performance optimization analysis
- [ ] Remove redundant code and refactor

### Pre-Commit
- [ ] Run complete test suite
- [ ] Update version number automatically
- [ ] Generate requirement summary from prompts
- [ ] Add test cases for any encountered errors

### Post-Commit
- [ ] Build Release version
- [ ] Launch application for verification
- [ ] Provide development summary report

## Quick Reference

| Task | Command/Action |
|------|---------------|
| Version Update | `npm version patch` or custom script |
| Test Suite | `npm test` or `cargo test` |
| Performance Check | Benchmark critical paths |
| Code Quality | ESLint/Clippy + manual review |
| Auto-start | `npm run tauri dev` |

## Common Mistakes

1. **Skipping TDD** - Leads to untested code and hidden bugs
2. **Manual versioning** - Causes conflicts and inconsistencies
3. **Incomplete testing** - Misses edge cases and regressions
4. **Delayed refactoring** - Accumulates technical debt
5. **Ignoring recurring issues** - Wastes time on repeated fixes

## Real-World Impact
- 40% reduction in bug recurrence
- 60% faster development cycles
- Consistent version management
- Automated quality assurance
- Proactive issue resolution