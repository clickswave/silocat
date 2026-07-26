<!-- Pull requests go to `dev`, not `main`. If this one targets `main`, please retarget it. -->

## What this changes

<!-- One or two sentences. What is different after this merges? -->

## Why

<!-- What problem does it solve? Link the issue if there is one. -->

## How it was tested

<!-- What did you actually run? "Compiles" is not testing. -->

## Checklist

- [ ] Targets the `dev` branch
- [ ] `services/api_switch` builds (`cargo build`)
- [ ] `services/web_server` builds (`npm run build`)
- [ ] Regenerated the sqlx cache if SQL changed (`cargo sqlx prepare`)
- [ ] No emoji in code, comments or commit messages
- [ ] Does not weaken the zero-knowledge guarantee: no plaintext, key material
      or password reaches the server
