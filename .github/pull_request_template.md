## Description
Please describe the changes proposed in this Pull Request and their motivation/context.

## Type of Change
- [ ] Bug fix (non-breaking change which fixes an issue)
- [ ] New feature (non-breaking change which adds functionality)
- [ ] Breaking change (fix or feature that would cause existing functionality to not work as expected)
- [ ] Refactor (non-breaking change that improves code quality)
- [ ] Documentation update

## Quality & Compliance Checklist
- [ ] **Developer Certificate of Origin (DCO)**: I have signed off all commits in this PR (`git commit -s`).
- [ ] **Code Formatting**: Code follows project style guidelines (`cargo fmt --all --check`).
- [ ] **Linting**: Code passes Clippy checks cleanly (`cargo clippy --workspace --all-targets --all-features -- -D warnings`).
- [ ] **Testing**: Added/updated unit and integration tests (`cargo test --all-targets --all-features`).
- [ ] **Unused Dependencies**: Verified no unused dependencies (`cargo machete`).
- [ ] **Documentation**: Updated relevant `README.md` files or architectural docs.
- [ ] **Telemetry & Tracing**: Maintained OpenTelemetry context propagation across async boundaries.

## Related Issues
Closes # (issue number)
