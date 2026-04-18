# Postgres Migration, Backup, And Restore Runbook

This runbook covers the local Docker Compose Postgres service used by Shipyard
and the same operational sequence to adapt for hosted Postgres.

## Scope

- Database: `shipyard`
- User: `shipyard`
- Local compose file: `deploy/docker-compose.yml`
- Migration files: `migrations/*.sql`
- Local service name: `postgres`

The Compose service mounts `migrations/` into `/docker-entrypoint-initdb.d`.
Those SQL files run automatically only when the Postgres data volume is empty.
Existing databases need migrations applied explicitly.

## Preflight

```bash
docker compose -f deploy/docker-compose.yml ps
docker compose -f deploy/docker-compose.yml exec postgres pg_isready -U shipyard -d shipyard
docker compose -f deploy/docker-compose.yml exec postgres psql -U shipyard -d shipyard -c '\dt'
```

Record the current Git SHA, target Git SHA, operator, and maintenance window in
the release notes before changing a shared environment.

## Backup

Create a custom-format backup before every migration or release:

```bash
mkdir -p tmp/backups
docker compose -f deploy/docker-compose.yml exec -T postgres \
  pg_dump -U shipyard -d shipyard --format=custom --no-owner --no-acl \
  > tmp/backups/shipyard-$(date -u +%Y%m%dT%H%M%SZ).dump
```

Verify the dump is readable:

```bash
pg_restore --list tmp/backups/shipyard-YYYYMMDDTHHMMSSZ.dump | head
```

If `pg_restore` is not installed on the host, run the verification in the
container:

```bash
docker compose -f deploy/docker-compose.yml exec -T postgres \
  pg_restore --list < tmp/backups/shipyard-YYYYMMDDTHHMMSSZ.dump | head
```

For production, store the backup outside the application host, encrypt it using
the platform standard, and record the object path in the release notes.

## Fresh Database Initialization

For a disposable local database:

```bash
docker compose -f deploy/docker-compose.yml down -v
docker compose -f deploy/docker-compose.yml up -d postgres
docker compose -f deploy/docker-compose.yml logs postgres
```

Success criteria:

- Postgres reaches healthy state.
- `CREATE TABLE` and `ALTER TABLE` statements from every migration complete.
- `\dt` shows the expected Shipyard tables.

Do not use `down -v` on any environment with data that must be preserved.

## Applying New Migrations To An Existing Database

There is not currently a migration runner with a schema history table. Until one
exists, apply only the new SQL files for the target release, in numeric order,
after taking a backup.

Example:

```bash
docker compose -f deploy/docker-compose.yml exec -T postgres \
  psql -U shipyard -d shipyard -v ON_ERROR_STOP=1 \
  < migrations/0005_example.sql
```

For multiple new migrations:

```bash
for migration in migrations/0005_example.sql migrations/0006_example.sql; do
  docker compose -f deploy/docker-compose.yml exec -T postgres \
    psql -U shipyard -d shipyard -v ON_ERROR_STOP=1 < "$migration"
done
```

Success criteria:

- Every `psql` invocation exits `0`.
- API, worker, and DVM services can start against the migrated database.
- The local smoke checklist in `docs/runbooks/local-compose-e2e-smoke.md`
  passes for the changed surface.

If a migration fails, stop and restore from the preflight backup unless the
failure is clearly from re-running an idempotent `IF EXISTS` or `IF NOT EXISTS`
statement.

## Restore Into A Fresh Local Database

Use this to test backups or recover a local environment:

```bash
docker compose -f deploy/docker-compose.yml down -v
docker compose -f deploy/docker-compose.yml up -d postgres
docker compose -f deploy/docker-compose.yml exec -T postgres \
  dropdb -U shipyard --if-exists shipyard
docker compose -f deploy/docker-compose.yml exec -T postgres \
  createdb -U shipyard shipyard
docker compose -f deploy/docker-compose.yml exec -T postgres \
  pg_restore -U shipyard -d shipyard --clean --if-exists --no-owner --no-acl \
  < tmp/backups/shipyard-YYYYMMDDTHHMMSSZ.dump
```

Validate the restore:

```bash
docker compose -f deploy/docker-compose.yml exec postgres \
  psql -U shipyard -d shipyard -c 'select count(*) from users;'
docker compose -f deploy/docker-compose.yml up -d api worker dvm web
```

## Production Restore Guardrails

- Restore into a new database instance when possible, then cut traffic over.
- Keep API, worker, and DVM stopped while restoring into the active database.
- Verify row counts for `users`, `accounts`, `publish_items`, `jobs`,
  `publish_attempts`, and `dvm_requests`.
- Verify `jobs_ready_idx`, `publish_items_due_idx`, and DVM request indexes are
  present after restore.
- Restart API first, then worker, then DVM, then web.

## Rollback Criteria

Rollback by restore when:

- A migration corrupts authorization, queue, publish item, job, or DVM request
  state.
- API startup fails because expected schema objects are missing.
- Worker or DVM processing fails due to incompatible schema.
- Smoke checks cannot create, sign, schedule, or inspect publish items.

After rollback, keep the failed database snapshot for debugging if storage
policy permits.
