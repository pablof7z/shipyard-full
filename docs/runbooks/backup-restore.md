# Backup and Restore Runbook

**Date:** 2026-04-18
**Milestone:** M13 — Deployment, Release, and Hardening
**Scope:** Shipyard Postgres database (`shipyard` database, `postgres` service)

---

## Overview

Shipyard uses Postgres 16 Alpine as its database. The database contains:
- User accounts and sessions
- Relay configurations
- Publish queues and scheduled items
- Proposal metadata and state transitions
- DVM request tracking and audit events
- Delegate relationships

Migrations are auto-applied on first start via `docker-entrypoint-initdb.d`. Incremental migrations are applied when the service starts and detects new `.sql` files.

**This runbook covers:**
1. Routine pg_dump backups
2. Docker volume backups
3. Restore from pg_dump
4. Disaster recovery (clean environment)
5. Key environment variables for recovery

---

## 1. Routine pg_dump Backups

### 1.1 Manual Backup

```bash
# From the shipyard root directory
cd /home/pablo/Work/shipyard

# Create a timestamped backup
docker compose -f deploy/docker-compose.yml exec postgres \
  pg_dump -U shipyard shipyard \
  > "backups/shipyard-$(date +%Y%m%d-%H%M%S).sql"
```

### 1.2 Cron-Style Scheduling (systemd timer)

Create a timer for weekly backups:

```bash
# /etc/systemd/system/shipyard-backup.timer
[Unit]
Description=Shipyard Postgres Backup Weekly

[Timer]
OnCalendar=Sun 03:00
Persistent=true

[Install]
WantedBy=timers.target
```

```bash
# /etc/systemd/system/shipyard-backup.service
[Unit]
Description=Shipyard Postgres Backup

[Service]
Type=oneshot
ExecStart=/bin/bash -c 'docker compose -f /home/pablo/Work/shipyard/deploy/docker-compose.yml exec -T postgres pg_dump -U shipyard shipyard > /var/backups/shipyard-$(date +%%Y%%m%%d-%%H%%M%%S).sql'
WorkingDirectory=/home/pablo/Work/shipyard
```

```bash
sudo systemctl enable shipyard-backup.timer
sudo systemctl start shipyard-backup.timer
```

### 1.3 Retention Policy

- Keep daily backups for **7 days**
- Keep weekly backups for **4 weeks**
- Keep monthly backups for **12 months**
- Example cleanup cron: `0 4 * * 0 find /var/backups/shipyard -name "*.sql" -mtime +28 -delete`

---

## 2. Docker Volume Backups

### 2.1 Identify the Volume

```bash
docker compose -f deploy/docker-compose.yml ps postgres
# Look for the volume mount: .../shipyard-postgres-data:/var/lib/postgresql/data

# Find the actual volume name
docker volume ls | grep shipyard
# shipyard_postgres-data
```

### 2.2 Snapshot the Volume

```bash
# Stop the postgres service first (optional but recommended for consistency)
cd /home/pablo/Work/shipyard
docker compose -f deploy/docker-compose.yml stop postgres

# Create a tar.gz snapshot
sudo tar -czf "backups/postgres-volume-$(date +%Y%m%d-%H%M%S).tar.gz" \
  /var/lib/docker/volumes/shipyard_postgres-data/_data

# Restart postgres
docker compose -f deploy/docker-compose.yml start postgres
```

### 2.3 Volume Location

```bash
# Find the actual filesystem path
docker volume inspect shipyard_postgres-data --format '{{.Mountpoint}}'
# Returns: /var/lib/docker/volumes/shipyard_postgres-data/_data
```

> **Note:** The Mountpoint path is only accessible to the `root` user by default. Use `sudo` to access it directly.

---

## 3. Restore from pg_dump

### 3.1 Restore to Same Instance

```bash
# Stop shipyard services (don't stop postgres)
cd /home/pablo/Work/shipyard
docker compose -f deploy/docker-compose.yml stop api worker dvm web

# Drop and recreate the database (destructive!)
docker compose -f deploy/docker-compose.yml exec postgres \
  psql -U shipyard -c "DROP DATABASE IF EXISTS shipyard;"
docker compose -f deploy/docker-compose.yml exec postgres \
  psql -U shipyard -c "CREATE DATABASE shipyard;"

# Restore from backup
docker compose -f deploy/docker-compose.yml exec -T postgres \
  psql -U shipyard shipyard < backups/shipyard-YYYYMMDD-HHMMSS.sql

# Restart services
docker compose -f deploy/docker-compose.yml start api worker dvm web
```

### 3.2 Restore to Fresh Environment

```bash
# 1. Start postgres only
docker compose -f deploy/docker-compose.yml up -d postgres
# Wait: docker compose -f deploy/docker-compose.yml ps (check "healthy")

# 2. Wait for migrations to run automatically
sleep 5

# 3. Restore from backup
docker compose -f deploy/docker-compose.yml exec -T postgres \
  psql -U shipyard shipyard < backups/shipyard-YYYYMMDD-HHMMSS.sql

# 4. Start remaining services
docker compose -f deploy/docker-compose.yml up -d
```

---

## 4. Disaster Recovery

### 4.1 Scenario: Volume Lost / Corrupt DB

```bash
cd /home/pablo/Work/shipyard

# 1. Destroy the corrupted volume
docker compose -f deploy/docker-compose.yml down -v  # -v removes volumes

# 2. Start fresh (migrations auto-apply)
docker compose -f deploy/docker-compose.yml up -d

# 3. Restore from latest pg_dump
docker compose -f deploy/docker-compose.yml exec -T postgres \
  psql -U shipyard shipyard < backups/shipyard-YYYYMMDD-HHMMSS.sql

# 4. Verify
docker compose -f deploy/docker-compose.yml exec postgres \
  psql -U shipyard shipyard -c "SELECT COUNT(*) FROM users;"
```

### 4.2 Scenario: No pg_dump Available

Recovery from migrations + latest state requires re-creating data from Nostr relays:

```bash
cd /home/pablo/Work/shipyard

# 1. Start with fresh migrations
docker compose -f deploy/docker-compose.yml down -v
docker compose -f deploy/docker-compose.yml up -d postgres
sleep 5  # wait for migrations

# 2. Start services (empty database — data will need to sync from relays)
docker compose -f deploy/docker-compose.yml up -d

# 3. User data re-populates from Nostr relays
#    Relays, proposals, and publish items sync through the normal workflow
```

> **Warning:** Without a pg_dump, user data (accounts, delegates, sessions) cannot be recovered. Only Nostr event data (proposals, publish items) can be re-synced from relays. This is why regular backups are critical.

---

## 5. Point-in-Time Recovery (PITR)

Postgres 16 supports PITR via WAL archiving. For full PITR:

### 5.1 Enable WAL Archiving

Add to `deploy/docker-compose.yml` under the `postgres` service:

```yaml
command: >
  postgres
  -c wal_level=replica
  -c max_wal_senders=3
  -c archive_mode=on
  -c archive_command=/bin/true  # Placeholder — configure for your backup system
```

> **Note:** Full PITR requires configuring an `archive_command` to copy WAL segments to durable storage (S3, NFS, etc.). This is infrastructure-dependent and not covered here.

### 5.2 Point-in-Time Restore

```bash
# Stop services
docker compose -f deploy/docker-compose.yml stop

# Use pg_restore with --point-in-time-recovery
# (Requires WAL archiving to be configured)
docker compose -f deploy/docker-compose.yml exec -T postgres \
  pg_restore -U shipyard -d shipyard --help | grep -i "point"

# Alternative: restore from pg_dump to a specific time
docker compose -f deploy/docker-compose.yml exec -T postgres \
  psql -U shipyard shipyard < backups/shipyard.sql
```

---

## 6. Key Environment Variables

These variables affect the database and must be set for recovery to work:

| Variable | Default | Notes |
|----------|---------|-------|
| `POSTGRES_DB` | `shipyard` | Database name |
| `POSTGRES_USER` | `shipyard` | Database user |
| `POSTGRES_PASSWORD` | `shipyard` | ⚠️ Change in production |
| `SHIPYARD_DATABASE_URL` | `postgres://...` | Connection string |
| `SHIPYARD_DVM_SECRET_KEY` | `1111...` (dev) | ⚠️ Rotate in production |

### 6.1 Rotation Procedure

To rotate credentials without downtime:

1. Take a pg_dump
2. Create new user: `psql -c "CREATE USER shipyard2 WITH PASSWORD 'newpass';"`
3. Grant same permissions as `shipyard` user
4. Update `SHIPYARD_DATABASE_URL` in docker-compose
5. Restart services (brief downtime)
6. Drop old user: `psql -c "DROP USER shipyard;"`

---

## 7. Verification Checklist

After any restore, verify:

```bash
# 1. Services are healthy
docker compose -f deploy/docker-compose.yml ps

# 2. Database is accessible
docker compose -f deploy/docker-compose.yml exec postgres \
  psql -U shipyard shipyard -c "SELECT 1;"

# 3. Migration state is correct
docker compose -f deploy/docker-compose.yml exec postgres \
  psql -U shipyard shipyard -c "SELECT version FROM schema_migrations ORDER BY version DESC LIMIT 1;"

# 4. API is responsive
curl http://localhost:8080/v1/status

# 5. Key tables have data
docker compose -f deploy/docker-compose.yml exec postgres \
  psql -U shipyard shipyard -c "SELECT COUNT(*) AS users FROM users; SELECT COUNT(*) AS sessions FROM sessions;"
```

---

## 8. Backup Storage Location

| Backup Type | Location | Retention |
|------------|----------|-----------|
| pg_dump SQL files | `/home/pablo/Work/shipyard/backups/` | 7 days (daily), 4 weeks (weekly) |
| Volume snapshots | `/var/backups/shipyard/` | 4 weeks |
| Off-site copies | S3 / cloud storage | 12 months |

> **Important:** Store at least one off-site copy of each backup. Volume snapshots on local disk are lost if the disk fails.

---

## Quick Reference

```bash
# Create backup
docker compose -f deploy/docker-compose.yml exec -T postgres pg_dump -U shipyard shipyard > backup.sql

# Restore
docker compose -f deploy/docker-compose.yml exec -T postgres psql -U shipyard shipyard < backup.sql

# Check volume location
docker volume inspect shipyard_postgres-data --format '{{.Mountpoint}}'

# Full disaster recovery
docker compose -f deploy/docker-compose.yml down -v
docker compose -f deploy/docker-compose.yml up -d postgres
sleep 5
docker compose -f deploy/docker-compose.yml exec -T postgres psql -U shipyard shipyard < backup.sql
docker compose -f deploy/docker-compose.yml up -d
```