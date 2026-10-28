# Seorakel

A cross-platform desktop tool that turns weekly SEO audit XML reports into a graphical dashboard with full history tracking.

Built with [egui](https://github.com/emilk/egui) / [eframe](https://github.com/emilk/egui/tree/master/eframe) — runs natively on Linux, macOS and Windows. No LLM, no network calls: all data comes from the XML files you import.

## Features

- **Import weekly XML reports** via file dialog or drag & drop (multiple files at once)
- **One tab per site** — every audited website gets its own dashboard tab; open tabs are restored between sessions
- **Multiple views per site**
  - **Overview** — overall score & grade, passed/warning/failure counts, category score bars, top critical issues, score trend
  - **Categories** — all 20 audit categories (core, technical, performance, links, images, security, crawl, schema, accessibility, content, social, E-E-A-T, URLs, mobile, i18n, legal, JS, redirects, HTML validation, GEO/AI search) with check counts and issue breakdowns
  - **Issues** — every issue with severity chip, rule id, affected pages, fix suggestion and details; filter by category and severity, free-text search, expandable rows
  - **History** — score-over-time chart, passed/warnings/failures chart, per-category trend lines, issue-count trend (appears once a site has 2+ weekly reports)
- **Report picker** — switch between weekly reports of the open site without leaving the tab
- **Dashboard library** — all imported sites with latest score/grade, report counts, open and delete actions, and a filter box
- **Persistent everything** — reports are copied into the app data folder, settings and the open-tab session are saved to disk, window size/position is remembered
- **Settings dialog** — dark/light mode, delete confirmation toggle, data folder location

## Data storage

All data lives in the platform data directory:

| Platform | Path |
|---|---|
| Linux | `~/.local/share/seorakel/` |
| macOS | `~/Library/Application Support/seorakel/` |
| Windows | `%APPDATA%\seorakel\data\` |

Layout:

```
seorakel/
├── library.json      # index of all sites (name, url, report count, latest score)
├── settings.json     # app settings
├── session.json      # open tabs + active tab/view
└── sites/
    └── <site-id>/
        ├── 2026-10-17.xml   # one file per weekly report, keyed by audit date
        └── 2026-10-24.xml
```

Deleting a dashboard from the library removes its stored reports as well.

## Usage

```sh
cargo run --release
```

Then import XML files via **📂 Import XML…** or by dragging them onto the window. Sample reports are included in [`examples/samples/`](examples/samples/).

## Development

```sh
cargo test          # unit tests (parser, store)
cargo clippy        # lints
cargo fmt           # formatting
cargo build --release
```

## XML format

The parser expects the `<seo-audit>` schema (schema="2"):

```xml
<seo-audit schema="2" url="https://example.com" score="88" grade="B" pages="34" date="2026-10-24" nonce="…">
  <summary passed="8464" warnings="1525" failures="311" notMeasured="988"/>
  <categories>
    <cat id="core" score="95" p="748" w="51" f="17"/>
    …
  </categories>
  <issues count="150" total="162" omitted="12" note="…">
    <issue severity="critical" rule="core-description-present" cat="core" pages="13" of="13">
      <on>https://… https://…</on>
      <msg>…</msg>
      <fix>…</fix>
      <details>…</details>
    </issue>
    …
  </issues>
  <not-measured></not-measured>
  <passed></passed>
</seo-audit>
```

## License

MIT
