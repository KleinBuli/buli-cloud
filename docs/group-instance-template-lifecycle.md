# Group, template and instance lifecycle

All mutations go through `CloudCore`. Manager accessors expose read operations;
manager mutations are crate-private so API callers cannot skip relationship checks.
Call `initialize()` before changing state.

## Persistent configuration

- `config/groups.toml` stores group names, paths, maintenance and ordered template
  assignments. Writes replace the complete file atomically; an unsuccessful write
  leaves the previous in-memory state intact. Invalid reloads retain existing groups.
- Each `templates/{group}/{template}/.bulicloud-template.toml` stores the template
  name, server software and optional Minecraft version. Minecraft server templates
  require a version; Velocity uses `velocity.jar`.
- Templates require the matching executable in the software cache when created.
  Missing executables produce an error without leaving a partial template.
- Legacy templates without metadata are migrated from one recognizable executable
  (`paper-{version}.jar`, `vanilla-{version}.jar` or `velocity.jar`). Ambiguous
  directories require explicit metadata. An old empty `global` template receives
  the configured fallback Paper executable.
- Creating a group also creates and registers its `global` template. Custom
  templates and instances must be removed before the group can be deleted.
  `global` cannot be deleted separately. Templates referenced by instances cannot
  be deleted, including when those instances are stopped.
- Ordinary write failures roll back configuration changes and staged removals.
  Directory operations and configuration commits are not a multi-file crash
  transaction. After an abrupt OS/process termination, inspect `.deleting-*`
  directories for recoverable files before manually removing them.

## Runtime instances

Instance creation copies exactly the selected group-scoped template and returns a
stopped instance. Starting is a separate operation; only the instance manager owns
child processes. `Running` means a live OS process, not Minecraft readiness.
Process exits are observed automatically. Stop sends `stop` (or `shutdown` for
Velocity), waits up to ten seconds, and force-terminates if necessary. Daemon
shutdown stops managed processes. Maintenance blocks creation and starting but
permits stopping and removal.

Dynamic instances are runtime objects, not persistent/static server definitions.
After a daemon restart, old running directories are preserved and their IDs are
skipped. Existing processes are not adopted. An OS crash or forced daemon kill can
leave external processes requiring manual inspection. Static-server creation and
recovery are not implemented by this change.

## API and CLI

- `POST /groups/{group}`
- `POST /templates/{group}/{template}` (optional `software=Paper|Vanilla|Velocity`
  and `version` query parameters; default is Paper with the configured fallback)
- `POST /instances/new/{group}/{template}` prepares a stopped instance
- `POST /instances/{id}/start` and `/stop`
- `DELETE /instances/{id}/remove`
- `DELETE /templates/{group}/{template}`
- `DELETE /groups/{group}`

CLI: `template create <group> <name> [--proxy]`, `start <group> <template>`.
A proxy executable must already be cached; automatic Velocity/Vanilla downloads
remain unimplemented. The existing CLI stop/copy/shutdown placeholders are outside
this lifecycle correction.

## Validation

`cargo test --workspace --offline` covers reloads, persisted metadata, failed-write
rollback, template scoping, maintenance, referential integrity, API routes and real
short-lived test child processes. It does not launch Java or a Minecraft server.
