# appScreener Pattern Sync

A Rust CLI utility that synchronizes local XML patterns with an existing Solar appScreener custom rule.

The local directory is treated as the source of truth. After a successful run, the rule contains exactly the same set of patterns as the specified local directory.

The utility does not create rules or modify rule metadata. It never calls `PUT /rules/custom`.

## Features

- read-only `plan` mode;
- full pattern-set synchronization;
- creation of new patterns;
- updates preserving existing pattern UUIDs;
- deletion of patterns missing from the local directory;
- XML fragment validation before contacting the server;
- mandatory severity and confidence;
- server snapshot before changes;
- verification before and after deletion;
- structured logging with `tracing`;
- JWT authentication through an environment variable;
- protection against accidentally removing every pattern.

## Requirements

- Rust 1.85 or newer;
- network access to the Solar appScreener API;
- a JWT belonging to a user allowed to modify custom rules;
- the UUID of an existing custom rule.

## Build

```powershell
cargo build --release
```

The executable is created at:

```text
target\release\appscreener-pattern-sync.exe
```

Run the project checks:

```powershell
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

## Local directory structure

```text
patterns/
├── patterns.yaml
├── P003-sensitive-source-arg2.xml
├── P004-sensitive-source-arg3.xml
├── P005-sensitive-source-arg4.xml
├── P100-sensitive-free-arg0-sink.xml
└── P200-sensitive-memory-sanitizer-arg0.xml
```

By default, the server-side pattern name is derived from the XML filename without its extension:

```text
P003-sensitive-source-arg2.xml
```

becomes:

```text
P003-sensitive-source-arg2
```

## Configuration

By default, the utility looks for the following manifest:

```text
<patterns-dir>\patterns.yaml
```

A different path can be specified with:

```text
--manifest <PATH>
```

### Minimal configuration

```yaml
version: 1

defaults:
  type: DATAFLOW
  severity: 3
  confidence: 1
  active: true

patterns: {}
```

These settings are applied to every XML file in the directory.

`severity` and `confidence` are mandatory. In the tested appScreener version, a pattern without these values may be stored in the database but will not participate in an analysis.

### Full configuration

```yaml
version: 1

defaults:
  type: DATAFLOW
  severity: 3
  confidence: 1
  active: true

patterns:
  P003-sensitive-source-arg2:
    name: Windows-sensitive-password-source-arg2
    severity: 3
    confidence: 2

  P004-sensitive-source-arg3:
    severity: 2
    confidence: 1
    active: true

  P005-sensitive-source-arg4.xml:
    type: DATAFLOW
    severity: 3
    confidence: 1
    active: true
    fileRegex: '.*\.(c|cc|cpp|cxx)$'

  reporting-pattern:
    type: REPORTING
    queryType: REGEX
    severity: 2
    confidence: 1
    active: true
    fileRegex: '.*\.cpp$'
```

A key under `patterns` can be either:

- a filename without `.xml`;
- a complete filename including `.xml`.

Both forms must not be specified for the same file.

## Pattern parameters

| Parameter | Required | Values | Description |
|---|---:|---|---|
| `name` | No | String | Overrides the filename-derived name |
| `type` | Yes | `DATAFLOW`, `REPORTING` | appScreener pattern type |
| `severity` | Yes | `0..3` | Severity level |
| `confidence` | Yes | Integer | Confidence level |
| `active` | No | `true`, `false` | Defaults to `true` |
| `queryType` | No | `REGEX`, `XPATH` | Query type |
| `fileRegex` | No | String | Source-file filter |

`type` and `queryType` are separate API fields:

```text
type:      DATAFLOW | REPORTING
queryType: REGEX | XPATH
```

For XML DataFlow patterns, `queryType` is normally omitted.

Settings for a particular pattern override values from `defaults`.

## XML fragments

The appScreener DataFlow DSL may contain multiple top-level sections:

```xml
<condition>
    <!-- ... -->
</condition>

<taintFlowChain>
    <!-- ... -->
</taintFlowChain>
```

Such content is not a conventional XML document with one root element.

For validation, the utility temporarily wraps the content in a synthetic root element. The original fragment is sent to appScreener without this synthetic root.

The following inputs are rejected:

- empty XML files;
- incorrectly closed elements;
- XML declarations;
- `DOCTYPE` declarations.

## Authentication

Using an environment variable is recommended:

```powershell
$env:APPSCREENER_TOKEN = "<JWT>"
```

Passing the JWT through `--token` is discouraged because it may appear in command history and process listings.

The token is never written to:

- snapshots;
- logs;
- synchronization plans;
- HTTP error messages.

## Plan mode

`plan` performs read-only requests and does not modify appScreener:

```powershell
target\release\appscreener-pattern-sync.exe plan `
  --base-url http://appscreener.example `
  --rule-id <RULE_UUID> `
  --patterns-dir C:\path\to\patterns
```

Example output:

```text
ACTION   PATTERN                                       DETAILS
-------- --------------------------------------------- ------------------------------
CREATE   P003-sensitive-source-arg2                    not present on server
UPDATE   P004-sensitive-source-arg3                    XML changed
SKIP     P005-sensitive-source-arg4                    already synchronized
DELETE   obsolete-pattern                             not present in local directory

Summary: create=1, update=1, skip=1, delete=1
```

Actions have the following meanings:

- `CREATE` — the local pattern does not exist on the server;
- `UPDATE` — a pattern with the same name exists, but its XML or parameters differ;
- `SKIP` — the pattern is already synchronized;
- `DELETE` — the server pattern does not exist in the local directory.

Pattern names are matched case-insensitively.

If the server contains several patterns with names that differ only by letter case, the operation fails because the match would be ambiguous.

## Apply mode

Always inspect the result of `plan` before applying changes.

```powershell
target\release\appscreener-pattern-sync.exe apply `
  --base-url http://appscreener.example `
  --rule-id <RULE_UUID> `
  --patterns-dir C:\path\to\patterns `
  --snapshot-out .\before-import.snapshot.json
```

A snapshot is mandatory and must not already exist. The utility intentionally refuses to overwrite an existing snapshot.

## Execution order

Changes are applied in the following safe order:

1. update existing patterns with `PUT`;
2. create new patterns with `POST`;
3. finalize every newly created pattern with `PUT`;
4. verify that the complete local set is present;
5. delete server patterns missing from the local directory;
6. verify the final state.

A newly created pattern is explicitly finalized with `PUT` because appScreener uses this request to register the pattern with the analysis engine.

If a `POST` or finalizing `PUT` fails, obsolete server patterns have not yet been deleted.

## Snapshot

The snapshot contains the original server-side pattern set:

- UUID;
- name;
- XML;
- `ruleId`;
- `type`;
- `severity`;
- `confidence`;
- `active`;
- `shared`;
- `user`;
- `queryType`;
- `fileRegex`.

Example:

```json
{
  "version": 1,
  "ruleId": "<RULE_UUID>",
  "patterns": [
    {
      "uuid": "<PATTERN_UUID>",
      "ruleId": "<RULE_UUID>",
      "severity": 3,
      "confidence": 1,
      "name": "example-pattern",
      "xml": "<condition>...</condition>",
      "type": "DATAFLOW",
      "active": true,
      "shared": false,
      "user": "username"
    }
  ]
}
```

An automatic restore command has not yet been implemented. A deleted pattern can be recreated from the snapshot, but it may receive a new UUID.

## Empty-directory protection

By default, the utility refuses to remove the complete server pattern set when the local directory contains no XML files:

```text
local directory contains no XML patterns
```

Intentional removal of all patterns requires:

```text
--allow-empty
```

Example:

```powershell
target\release\appscreener-pattern-sync.exe apply `
  --base-url http://appscreener.example `
  --rule-id <RULE_UUID> `
  --patterns-dir C:\path\to\empty-patterns `
  --snapshot-out .\before-cleanup.snapshot.json `
  --allow-empty
```

## Logging

The default log level is `INFO`.

```powershell
appscreener-pattern-sync.exe plan ...
```

Enable debug logging:

```powershell
appscreener-pattern-sync.exe -v plan ...
```

Enable maximum verbosity:

```powershell
appscreener-pattern-sync.exe -vv plan ...
```

Display errors only:

```powershell
appscreener-pattern-sync.exe -q plan ...
```

The `RUST_LOG` environment variable is also supported:

```powershell
$env:RUST_LOG = "appscreener_pattern_sync=debug"
```

The synchronization plan is written to `stdout`, while technical logs are written to `stderr`:

```powershell
appscreener-pattern-sync.exe plan ... > plan.txt
```

Full XML content is not written to the log. Pattern comparison uses the content size and SHA-256 hash.

## API methods

The utility uses the following endpoints:

```text
GET    /rules/custom/{id}/info
GET    /rules/custom/{ruleId}/patterns
POST   /patterns/pattern
PUT    /patterns/pattern
DELETE /patterns/pattern?uuid={uuid}
```

Rule metadata is never modified. The following endpoint is not called:

```text
PUT /rules/custom
```

## Post-import verification

After a successful `apply`, run `plan` again:

```powershell
target\release\appscreener-pattern-sync.exe plan `
  --base-url http://appscreener.example `
  --rule-id <RULE_UUID> `
  --patterns-dir C:\path\to\patterns
```

Expected result:

```text
Summary: create=0, update=0, skip=5, delete=0

The rule already matches the local directory.
```

Every operational pattern should contain at least the following fields in the appScreener response:

```json
{
  "severity": 3,
  "confidence": 1,
  "type": "DATAFLOW",
  "active": true
}
```

## Troubleshooting

### HTTP 401

The JWT is missing, expired, or revoked:

```text
check APPSCREENER_TOKEN
```

Issue a new JWT and update the environment variable.

### HTTP 403

The current user does not have permission to modify the custom rule.

### HTTP 500

The utility prints a length-limited appScreener response body. Check:

- mandatory `severity` and `confidence`;
- the `type` value;
- XML DSL validity;
- whether the UUID belongs to a custom rule.

### Snapshot already exists

The utility does not overwrite backups:

```text
the file must not already exist
```

Specify a new filename:

```text
--snapshot-out .\before-import-02.snapshot.json
```

### Pattern exists but does not participate in analysis

Confirm that the server response contains:

```json
{
  "severity": 3,
  "confidence": 1,
  "active": true
}
```

A newly created pattern must complete both requests:

```text
POST /patterns/pattern
PUT  /patterns/pattern
```

The second request corresponds to clicking **Save** in the appScreener UI.

## Limitations

- The appScreener API does not provide batch transactions.
- Automatic rollback is not implemented.
- Restored patterns may receive different UUIDs.
- The utility works only with an existing custom rule.
- Creating rules and changing rule metadata are not supported.
- Only local `*.xml` files are supported.
- The allowed `confidence` range is not defined by OpenAPI and is controlled by configuration.