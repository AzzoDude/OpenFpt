# OpenFpt

A Rust CLI scraper for the FuOverflow community forum.

## Features

- Log in and persist the session (`session.txt` next to the executable)
- Search every thread of a subject, with prefix filtering (`FE`, `PE`)
- Inspect a thread's attachment list
- Install a thread or an entire subject: download full-resolution attachments and
  write a `manifest.toml` + `comments.toml` per thread
- Detects the site's automatic ban page and reports it instead of failing silently

## Build

```sh
cargo build --release --workspace
```

The binary is `target/release/OpenFpt.exe`.

## Usage

```sh
OpenFpt.exe login                          # prompts for login + password
OpenFpt.exe login <user> <pass>            # non-interactive
OpenFpt.exe search PRF192                  # all threads of a subject
OpenFpt.exe search PRF192 FE               # only "Đề Thi FE" threads
OpenFpt.exe thread 7218                    # attachment table of one thread
OpenFpt.exe thread 7218 --comments         # + comments
OpenFpt.exe install 7218                   # download one thread
OpenFpt.exe install PRF192                 # download every thread of a subject
OpenFpt.exe install PRF192 --no-comments   # attachments only (much faster)
OpenFpt.exe logout                         # delete the saved session
```

### Options

- `--dir <folder>` — download root (`~` is expanded), default `install`
- `--delay-ms <ms>` — pause between requests, default `500`
- `--no-comments` — skip comment fetching during `install`

## Notes

- Comments and full-resolution files require a premium account.
- Keep `--delay-ms` high and prefer single-thread installs; the site bans
  accounts that make too many requests in a row.
- `session.txt` contains your session cookies — don't commit it (already in `.gitignore`).

## License

GPL-3.0-only
