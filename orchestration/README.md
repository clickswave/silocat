# Orchestration

**Self-hosting? Use `docker-compose.selfhost.yml`.** It is the only file here
that stands alone: it brings its own Postgres and MinIO, so a laptop or a single
VPS is enough.

```bash
cp ../env.selfhost.example .env
docker compose -f docker-compose.selfhost.yml --env-file .env up -d
```

The other two files are how Clickswave runs the hosted silo.cat, kept here so
the deployment is inspectable alongside the code. They will not work outside
that environment: both expect an external `clickswave_network` and a shared
Postgres that live elsewhere in the private monorepo.

| File | What it is |
|---|---|
| `docker-compose.selfhost.yml` | Self-contained. Start here. |
| `docker-compose.dev.yml` | Clickswave's local development stack |
| `docker-compose.prod.yml` | Clickswave's production stack |
