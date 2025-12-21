# Leptos 0.7 Migration - Error Tracking Log

## Session 1: 2025-12-22 - Signal & Resource API Fixes

### Session Start
- **Errors**: 226
- **Target**: < 20
- **Scope**: Priority 1 (Signal & Resource APIs)

### Fixes Applied

#### Fix 1: resizable_panels.rs WriteSignal API
```rust
// Before (WRONG - E0618)
set_is_dragging(true)
set_left_width(clamped)

// After (CORRECT - Leptos 0.7)
set_is_dragging.set(true)
set_left_width.set(clamped)
```
Lines affected: 77, 106, 115, 131, 136
Error reduction: ~5 errors

#### Fix 2: tooltip.rs Signal getter/setter API  
```rust
// Before (WRONG - E0618)
set_hovering(true)
is_hovering()

// After (CORRECT)
set_hovering.set(true)
is_hovering.get()
```
Lines affected: 72, 73, 78
Error reduction: ~3 errors

#### Fix 3: query_timeline.rs ReadSignal getter
```rust
// Before (WRONG - E0277)
if expanded() { ... }

// After (CORRECT)
if expanded.get() { ... }
```
Lines affected: 215, 285
Error reduction: ~2 errors

#### Fix 4: tool_invocation_list.rs ReadSignal getter
```rust
// Before (WRONG - E0277)
if expanded() { ... }

// After (CORRECT)
if expanded.get() { ... }
```
Lines affected: 125, 162
Error reduction: ~2 errors

#### Fix 5: state_machine_trace.rs ReadSignal getter
```rust
// Before (WRONG - E0277)
if show_details() { ... }
show_details=show_details()

// After (CORRECT)
if show_details.get() { ... }
show_details=show_details.get()
```
Lines affected: 53, 76
Error reduction: ~2 errors

### Session End
- **Errors**: 208
- **Errors Fixed**: 18 (8% reduction)
- **Files Modified**: 5
- **Categories Improved**:
  - E0618: 23 → 6 (-17 errors) ✅
  - E0599: 36 → 26 (-10 errors) ✅
  - E0277: 31 → 30 (-1 error) ✅
  - E0560: 2 → 0 (-2 errors) ✅

### Remaining Work

**High Priority** (causes 124 errors):
- [ ] Refactor conditional view rendering using `<Show>` component
- [ ] Files affected: ~40 components with if/match in views

**Medium Priority** (causes 11 errors):
- [ ] Review closure signatures in event handlers
- [ ] Fix argument count mismatches

**Low Priority** (causes 7 errors):
- [ ] Address move/borrow issues
- [ ] Review value ownership and cloning

**Blocked by Type System** (causes 3 errors):
- [ ] Resolve type inference issues
- [ ] Add explicit type annotations where needed

## Key Learnings

### Leptos 0.7 Breaking Changes
1. **Signal getter/setter API**
   - Old: `signal()` to read, `set_signal(value)` to write
   - New: `signal.get()` to read, `set_signal.set(value)` to write

2. **RwSignal creation**
   - Old: `create_rw_signal(value)`
   - New: `RwSignal::new(value)` ✅ Already migrated

3. **View type system**
   - Requires all branches to return same type
   - Solution: Use `<Show>` component for conditionals

### Code Patterns Fixed
- Signal access in view! macros: 9 instances
- WriteSignal setter calls: 8 instances  
- ReadSignal getter calls: 9 instances

## Error Categories Reference

| Code | Category | Count | Status |
|------|----------|-------|--------|
| E0308 | Type Mismatch (Views) | 124 | Needs View composition refactoring |
| E0599 | Missing Method | 26 | Partially fixed |
| E0277 | Trait Bound | 30 | Partially fixed |
| E0618 | Not Callable | 6 | Significantly improved ✅ |
| E0593 | Closure Args | 11 | Needs investigation |
| E0382 | Move/Borrow | 7 | Needs investigation |
| E0369 | Invalid Variant | 1 | Needs investigation |
| E0283 | Ambiguous Type | 2 | Needs investigation |
| E0282 | Unable to Infer | 1 | Needs investigation |

## Next Session Recommendations

1. **Priority**: Tackle E0308 errors with View composition refactoring
   - High impact (124 errors)
   - Clear solution (use `<Show>` instead of `if`)
   - Can be automated with find/replace + manual verification

2. **Timeline**: 2-3 hours for E0308 refactoring
   - Review patterns in 40+ files
   - Apply `<Show>` wrapper pattern
   - Test and verify

3. **Follow-up**: E0593 and E0382 errors need case-by-case review
   - May reveal design issues to address
   - Recommend one file at a time

## Success Metrics

- [x] Signal API patterns corrected
- [x] RwSignal usage verified
- [ ] All E0308 errors resolved (target: next session)
- [ ] E0618 errors reduced to < 2 (current: 6)
- [ ] Overall errors < 20 (current: 208)

---

*Last Updated: 2025-12-22*
*Next Review: After E0308 refactoring session*
