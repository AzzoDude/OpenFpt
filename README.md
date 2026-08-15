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

The binary is `target/release/openfpt.exe` (`openfpt` on Unix).

## Usage

```sh
openfpt login                          # prompts for login + password
openfpt login <user> <pass>            # non-interactive
openfpt search PRF192                  # all threads of a subject
openfpt search PRF192 FE               # only "Đề Thi FE" threads
openfpt thread 7218                    # attachment table of one thread
openfpt thread 7218 --comments         # + comments
openfpt install 7218                   # download one thread
openfpt install PRF192                 # download every thread of a subject
openfpt install PRF192 --no-comments   # attachments only (much faster)
openfpt logout                         # delete the saved session
```

> On Windows, `openfpt` resolves only when the folder containing `openfpt.exe`
> is on your `PATH` (winget installs it this way). Otherwise type `openfpt.exe`.

### Options

- `--dir <folder>` — download root (`~` is expanded), default `install`
- `--delay-ms <ms>` — pause between requests, default `500`
- `--no-comments` — skip comment fetching during `install`

## Notes

- Comments and full-resolution files require a premium account.
- Keep `--delay-ms` high and prefer single-thread installs; the site bans
  accounts that make too many requests in a row.
- `session.txt` contains your session cookies — don't commit it (already in `.gitignore`).
  It is saved next to the executable, so with a winget install you'll find it
  under `%LOCALAPPDATA%\Microsoft\WinGet\Packages\`.

## Install via winget

The package is published to the Windows Package Manager community repository
as `AzzoDude.OpenFpt`:

```sh
winget install AzzoDude.OpenFpt
```

After install, `openfpt` is on your `PATH`, so `openfpt install PRF192` just works.

Publishing is automated by the `Release` workflow:

1. Tag a version: `git tag v0.1.0 && git push origin v0.1.0`
2. The workflow builds `openfpt.exe`, creates a GitHub Release with it, then
   runs `winget-releaser` to open a PR against `microsoft/winget-pkgs`.
3. Once the PR is merged (usually within a day), run `winget install AzzoDude.OpenFpt`.

### Prerequisites (one-time)

- A [GitHub PAT](https://github.com/settings/tokens) with the `public_repo`
  scope, stored as the `WINGET_TOKEN` secret
  (Settings → Secrets and variables → Actions).
- The `Release` workflow runs on tags; if the winget publish step fails or the
  PR needs manual help, create the manifest yourself with
  [`wingetcreate`](https://github.com/microsoft/winget-create):

  ```sh
  wingetcreate update AzzoDude.OpenFpt --urls https://github.com/AzzoDude/openfpt/releases/download/v0.1.0/openfpt.exe --version 0.1.0
  ```

## License

GPL-3.0-only
