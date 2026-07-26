# Contributing

Thanks for looking. Silocat is a small project, so the process is short.

## Where to send pull requests

**Open pull requests against `dev`, not `main`.**

`main` is what runs at silo.cat. `dev` is where work lands first and gets
released to `main` in batches. A pull request against `main` will be asked to
retarget, which wastes a round trip.

```bash
git clone https://github.com/clickswave/silocat
cd silocat
git checkout dev
git checkout -b your-change
```

## Before you start

For anything beyond a bug fix or a typo, open an issue first and say what you
intend to do. It is a zero-knowledge product, so changes that touch the crypto,
the share links, or the auth path need a design conversation before code, not
after.

**Do not open an issue for a security problem.** Read
[SECURITY.md](SECURITY.md) and email team@silo.cat instead.

## Running it

The self-host compose file is fully self-contained, including Postgres and
MinIO, so you do not need any Clickswave infrastructure:

```bash
cp env.selfhost.example .env
docker compose -f orchestration/docker-compose.selfhost.yml --env-file .env up -d
```

The app comes up at `http://localhost:12001`. See
[orchestration/README.md](orchestration/README.md) for what the other compose
files are.

## What CI checks

- `services/api_switch` builds. It compiles against a checked-in sqlx query
  cache, so if you change SQL, regenerate it with `cargo sqlx prepare` or the
  build fails.
- `services/web_server` builds.
- Lint runs but does not block. There is a backlog of type-inference warnings;
  please do not add to it, but you are not expected to fix it either.

## House style

- Match the surrounding code. Comment density, naming and idiom vary a little
  between the Rust and the Svelte side; follow whichever file you are in.
- Comments should explain why something is the way it is, not restate what the
  line does.
- No emoji, in code, comments, commit messages or documentation.
- Write commit messages that say what changed and why. One line, then a blank
  line, then detail if it needs it.

## Licence

Contributions are accepted under [AGPL-3.0](LICENSE), the same licence as the
project. If you run a modified Silocat as a network service you have to offer
your users the modified source, and that applies to contributors too.
