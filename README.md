# Slopcheck

A CLI tool that checks for indicators of AI-generated code in a project and its dependencies.

![Screenshot of running slopcheck on Rolldown. There are a lot of matches.](demo.png)

Currently, checking dependencies is only implemented for Rust (using `cargo metadata`) and JavaScript (by naively parsing the `package.json` and recursively fetching npm).
Also, be aware that to simplify implementation, Slopcheck mostly assumes that dependencies are on the latest version.

The cache for cloned repositories and some metadata is at `~/.cache/slopcheck` or [your operating systems's cache directory](https://docs.rs/dirs/latest/dirs/fn.cache_dir.html). Items in this cache are updated after 24 hours.

It is not advised to run Slopcheck on untrusted projects, as it may request arbitrary sources and possibly run build scripts.

## Features

- Shows whether a repository has commits by a known LLM (Claude, Copilot, etc).
- Looks for the presence of files like `CLAUDE.md`, `AGENTS.md`, etc in the working tree or in the `.gitignore` (for projects trying to hide them).
- Checks all dependencies and displays if they have indicators of AI too.
- Distinguishes between current and former LLM use.

## Usage

```sh
cargo install --git https://github.com/mat-1/slopcheck
slopcheck ./something
```
