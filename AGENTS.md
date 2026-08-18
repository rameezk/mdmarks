# AGENTS.md

## Agent skills

### Issue tracker

Issues and specs live as GitHub issues in `rameezk/mdmarks`, managed with the `gh` CLI. See `docs/agents/issue-tracker.md`.

### Triage labels

The five canonical triage roles, each label string equal to its name. See `docs/agents/triage-labels.md`.

### Domain docs

Single-context: one `CONTEXT.md` + `docs/adr/` at the repo root. See `docs/agents/domain.md`.

## Git workflow

- Never commit directly to `main`. Do work on a feature branch (e.g. `feat/<issue>-<slug>`, `fix/<issue>-<slug>`) and commit there.
- One commit per PR. Squash the branch into a single commit before opening the PR.
- Commit messages follow [Conventional Commits](https://www.conventionalcommits.org): `<type>[optional scope]: <description>`, where `type` is one of `feat`, `fix`, `docs`, `refactor`, `test`, `chore`, etc.
- When implementing an issue, include a closing keyword in the commit body (e.g. `Closes #<issue>`) so the issue closes automatically when the PR merges.
