# omah — User Manual Testing

Build omah before testing:

```sh
cargo build --release
alias omah="$(pwd)/target/release/omah"
```

Use a temp directory for all test runs to avoid touching real config:

```sh
T="$(mktemp -d)"
```

---

## Test Results

All 56 test cases passed. See `git log` for test execution details.

| Area | TC IDs | Count | Status |
|------|--------|-------|--------|
| Init | 001, 002, 054 | 3 | ✅ |
| Config management (add/remove/list/info) | 003–010 | 8 | ✅ |
| Backup | 011–020, 041–043, 052, 056 | 14 | ✅ |
| Restore | 021–025, 044, 050, 051 | 8 | ✅ |
| Status | 026–028, 048 | 4 | ✅ |
| Diff | 029–033 | 5 | ✅ |
| Setup & deps | 047–049, 051 | 4 | ✅ |
| Config overrides | 034–036, 055 | 4 | ✅ |
| Edge cases | 037–040, 045–046 | 6 | ✅ |
| No-args / help | 053 | 1 | ✅ |

**Total: 56 test cases — ALL PASS**
