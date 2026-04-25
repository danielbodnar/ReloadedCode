# Architecture: reloaded-code-bubblewrap

Linux-only library that builds [bubblewrap] sandbox profiles, probes host
capabilities, and produces wrapped command lines.

For the security model, see [Extra Sandboxing Notes](https://reloaded-project.github.io/ReloadedCode/extra-sandboxing-notes/).

## File Map

```text
reloaded-code-bubblewrap
├── lib.rs                  crate root, re-exports, Linux-only gate
├── error.rs                LinuxBwrapError
├── probe.rs                bwrap detection & shell resolution (cached)
├── path_util.rs            normalize_path helper
├── profile/
│   ├── mod.rs              module root; re-exports public API surface
│   ├── types.rs            Profile, Preset, TmpBacking, Availability, etc.
│   ├── builder.rs          Builder + build() + static arg precomputation
│   ├── factory.rs          create_sandbox, create_sandbox_with, create_temp_sandbox, SandboxDirs, CreateSandboxError, TempSandboxDirs
│   ├── presets.rs          public_bot() & trusted_maintenance() constructors
│   ├── validation.rs       path/symlink/env/tmp validators
│   └── layout.rs           SandboxLayout - "is this host path visible inside?"
├── wrap/
│   ├── mod.rs              module root; cfg(feature) gates, re-exports
│   ├── command.rs          wrap_command → LinuxBwrapWrappedCommand
│   ├── tokio.rs            async CommandWrap  (feature "tokio")
│   └── blocking.rs         sync CommandWrap   (feature "blocking")
└── test_helpers.rs         fake bwrap/shell fixtures (cfg(test))
```

## Building a Profile

```text
   Builder::public_bot()          Builder::trusted_maintenance()         Builder::new()
   or any with_*() chain
           │
           │  .build()
           ▼
   ┌─────────────────────────────────────────────────────────┐
   │  0. ensure cache root subdirs (when cache root mounted) │
   │  1. validate paths, env, symlinks, tmp, creds,          │
   │     mounts, tmpfs overlays                              │
   │  2. resolve bwrap binary  (probe.rs, cached)            │
   │  3. resolve visible shell (builder.rs + layout.rs)      │
   │  4. precompute static bwrap argv                        │
   └───────────────────────────┬─────────────────────────────┘
                              │
                              ▼
                         ┌──────────┐
                         │ Profile  │  frozen, Clone, thread-safe
                         └──────────┘
```

## Using a Profile

Pass the `Profile` to `wrap_command` directly, or use an adapter:

```text
                ┌──────────┐
                │ Profile  │
                └────┬─────┘
                     │
          ┌──────────┼──────────┐
          ▼          │          ▼
  tokio::            │   blocking::
  build_command_wrap │   build_command_wrap
  (async CommandWrap)│   (sync CommandWrap)
          │          │          │
          └──────────┼──────────┘
                     ▼
              wrap_command()  →  LinuxBwrapWrappedCommand (argv iterator)
```

## What build() Does

```text
   ensure cache dirs ──► validate ──► find bwrap ──► find shell ──► build static args
          │                │              │               │                │
    validation.rs    validation.rs    probe.rs    builder.rs +      builder.rs
    (when cache      + builder.rs   (cached)     layout.rs        (one-time
     root mounted)   (creds)                     (visibility)      precompute)
```

Result: a `Profile` with `static_args: Arc<[OsString]>` containing the full
bwrap prefix (flags, mounts, env). `wrap_command` only appends `--chdir
<cwd> -- <shell> -c <cmd>`.

## Two Presets at a Glance

|                    | PublicBot                  | TrustedMaintenance             |
| ------------------ | -------------------------- | ------------------------------ |
| Network            | off (`--unshare-net`)      | on                             |
| Host filesystem    | selective read-only mounts | full `/` read-only             |
| Writable areas     | workspace, home, `/tmp`    | workspace, home, cache, `/tmp` |
| `/etc/shadow`      | hidden (not mounted)       | hidden (file overlay)          |
| Cache root         | not mounted                | bind-mounted                   |
| Env                | cleared, sanitized PATH    | cleared, PATH + XDG + TMPDIR   |
| Safe for untrusted | yes                        | no                             |

## Path Visibility (layout.rs)

When `wrap_command` needs to translate a host working directory to a sandbox
path, `SandboxLayout::classify` walks the mount tree:

```text
host_path
 ├── under workspace?       → remap workspace → workspace_dest
 ├── under synthetic_home?  → remap home → home_dest
 ├── under BindHost tmp?    → remap to /tmp
 ├── under cache_root?      → same path (if mounted)
 ├── in ro/rw mounts?       → same path
 ├── ro-host-rootfs?        → same path (unless tmpfs overlay hides it)
 └── else                   → hidden (error)
```

Same logic is used at `build()` time to find a shell that's actually visible
inside the sandbox.

## Probe Cache (probe.rs)

`probe_backend_uncached()` spawns `bwrap --version` then a minimal sandbox to verify
namespace support. `probe_backend()` caches results in a `OnceLock<RwLock<...>>`
keyed on `$PATH` - a changed PATH invalidates the cache.

Shell search order: `bash` on PATH → `sh` on PATH → hardcoded candidates
(Nix, FHS) → deduplicated by resolved path.

## Error Model

```text
LinuxBwrapError
├── InvalidPath(String)    bad path, bad env name/value, bad symlink,
│                          bad credential mount, bad tmp backing,
│                          cache subdir I/O failure, invisible workdir
└── Execution(String)      bwrap missing, bwrap broken, no visible shell, unavailable
```

All validation fails at `build()`. `wrap_command` can only fail on a bad
per-call working directory.

## Feature Flags

```text
(default)   → wrap_command only (no process-wrap dependency)
tokio       → wrap::tokio::build_command_wrap   (process-wrap tokio1, process-group)
blocking    → wrap::blocking::build_command_wrap (process-wrap std, process-group)
```

Both execution adapters set stdin=null, stdout/stderr=piped, and wrap with
`ProcessGroup::leader()` for clean signal handling.

## Testing

Fake `bwrap` and `bash` scripts in temp dirs with managed `$PATH`. Tests
that touch `$PATH` run `#[serial]` to avoid cache contamination. No real
bubblewrap installation needed.

[bubblewrap]: https://github.com/containers/bubblewrap
