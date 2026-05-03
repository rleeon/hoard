<!-- Thanks for the PR! A few quick checks first. -->

## What does this change?

<!-- One paragraph. Why is this needed? -->

## Linked issue

Closes #

## Checklist

- [ ] I ran `cargo fmt --all`, `cargo clippy --workspace -- -D warnings`,
      and `cargo test --workspace`.
- [ ] If I added/changed a `query!` or `query_as!`, I regenerated and
      committed `.sqlx/`.
- [ ] If I added a new SQL migration, I did **not** modify any existing
      released migration.
- [ ] I updated `CHANGELOG.md` under `[Unreleased]` if this is
      user-visible.
- [ ] I updated docs (`docs/`, `README.md`) if this changes user-facing
      behavior.
