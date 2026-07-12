# (deploy drop-zone) health_monitor binary

This directory intentionally holds no source. The health_monitor binary is built
once, off-box, from the canonical project at `projects/health_monitor` and
uploaded to R2 (`scripts/build_health_monitor.py`). At deploy the ansible role
downloads it here as `target/release/health_monitor`; the compose service runs
that prebuilt binary on a `debian:bookworm-slim` base — exactly the api_switch
pattern. Local-dev/standalone builds use `projects/health_monitor/Dockerfile`.
