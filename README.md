# SEO for Korean — Keyword Gateway

Korean text-analysis gateway for the [SEO for Korean](https://github.com/dalsoop/seo-for-korean) WordPress plugin.

## Status

- **V1 (current):** regex-based keyword matching with hard-coded particle list. Same strategy as the plugin's PHP fallback, but centralized — one place to upgrade.
- **V2 (planned):** lindera + ko-dic morphological tokenization. Blocked on lindera 0.32's ko-dic asset hosting (the build script's S3 URL currently 404s). Tracking: see `TODO` at the top of `src/main.rs`.

The point of the gateway isn't the engine — it's that the engine can be upgraded once and benefit every WP install.

## Endpoints

| Method | Path                | Body                        | Returns                         |
|--------|---------------------|-----------------------------|---------------------------------|
| GET    | `/health`           | —                           | `{status, service, version, engine}` |
| POST   | `/keyword/contains` | `{text, keyword}`           | `{count, matches[], engine}`    |

`engine` is `"regex"` for V1, will become `"lindera"` once morphology lands.

### Example

```bash
curl -s http://localhost:8787/keyword/contains \
  -H 'content-type: application/json' \
  -d '{"text":"워드프레스를 사용하면 좋습니다. 워드프레스의 장점은 많아요.","keyword":"워드프레스"}'
```

```json
{
  "count": 2,
  "matches": ["워드프레스를", "워드프레스의"],
  "engine": "regex"
}
```

## Run

```bash
cargo run --release
# or with custom bind:
BIND=0.0.0.0:8787 cargo run --release
```

V1 build is fast (~20s release build, no embedded dictionary). V2 with lindera+ko-dic will be heavier.

## Deploy

Single static binary, no runtime dependencies. Drop on any Linux host with `glibc`. Recommended: systemd unit on a small LXC. See `deploy/seo-for-korean-gateway.service`.

A `Dockerfile` is also included (multi-stage, Debian trixie-slim runtime).

## Why a separate service

The PHP plugin runs inside whatever WP host the user picked — could be shared hosting, could be missing PHP extensions, will not have a Korean morphology stack installed. Pushing analysis to a single shared service means:

- One install of the engine for all sites
- Plugin stays small (single zip, no native deps)
- Engine can be upgraded independently of every WP install
- Plugin falls back gracefully when the gateway is unreachable

## License

GPL-2.0-or-later. Same as the plugin.
