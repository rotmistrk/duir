# Epic: S3 Storage Backend

**ID**: 03
**Priority**: P2
**Status**: backlog

## Goal

Add S3 as a storage backend. Each `.todo.json` file maps to one S3 object.
Sync on open/save with configurable bucket and prefix.

## Acceptance Criteria

- [ ] S3 storage adapter implements the storage trait from Epic 01
- [ ] Download file from S3 on open
- [ ] Upload file to S3 on save
- [ ] List available files under S3 prefix
- [ ] Sync all files on startup (optional)
- [ ] Configuration via config file or environment variables
- [ ] Works with S3-compatible stores (MinIO, etc.)

## Stories

- [ ] 03.001 — S3 storage adapter
- [ ] 03.002 — Configuration and credentials
- [ ] 03.003 — Sync workflow

## Notes

- Use aws-sdk-s3 crate
- S3 versioning handles backup — no need for custom versioning
- Credentials via standard AWS chain (env, profile, instance role)
