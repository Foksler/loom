# Loom Coding Assistant

## IMPORTANT

- ALWAYS author property based tests and when doing do add 'why the test is important and what its purpose is' to the test documentation.
- ALWAYS use structured logging

## Development loop

```bash
make build    # Build workspace
make test     # Run all tests
make lint     # Run clippy
make fix      # Auto-fix clippy + format
make check    # Full CI check (format + lint + build + test)
```


