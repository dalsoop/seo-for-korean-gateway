# SEO for Korean — Morphology Gateway

Korean text-analysis gateway for the [SEO for Korean](https://github.com/dalsoop/seo-for-korean) WordPress plugin.

## Status

**v0.2.0** — real morphological tokenization via **lindera** + **mecab-ko-dic** (embedded at compile time).

The plugin's PHP fallback uses a regex over a hard-coded list of 25 particles, which catches ~80% of cases. The gateway's lindera engine catches everything: compound particles, conjugation forms, novel morphemes, mid-word hits the regex can't find. Same input, more accurate output, single upgrade benefits every WP install.

## Endpoints

| Method | Path                  | Body                | Returns                                    |
|--------|-----------------------|---------------------|--------------------------------------------|
| GET    | `/health`             | —                   | `{status, service, version, engine}`       |
| POST   | `/keyword/contains`   | `{text, keyword}`   | `{count, matches[], engine}`               |
| POST   | `/morphology/tokenize`| `{text}`            | `{tokens[{surface,pos}], nouns[], engine}` |
| POST   | `/analyze`            | `{title, content,…}`| `{score, grade, checks[], engine}`         |

`engine` is now always `"lindera"` (was `"regex"` in v0.1).

### Tokenization example

```bash
curl -s http://localhost:8787/morphology/tokenize \
  -H 'content-type: application/json' \
  -d '{"text":"워드프레스를 사용하면 좋습니다"}'
```

```json
{
  "tokens": [
    { "surface": "워드프레스", "pos": "NNP" },
    { "surface": "를",         "pos": "JKO" },
    { "surface": "사용",       "pos": "NNG" },
    { "surface": "하",         "pos": "XSV" },
    { "surface": "면",         "pos": "EC"  }
  ],
  "nouns": ["워드프레스", "사용"],
  "engine": "lindera"
}
```

### Keyword matching example

The keyword is tokenized too, so we walk text-token sequences looking for the keyword's token sequence. Particles fall away because lindera segments them as separate tokens — the regex fallback would have included them in the matched surface.

```bash
curl -s http://localhost:8787/keyword/contains \
  -H 'content-type: application/json' \
  -d '{"text":"워드프레스를 사용. 워드프레스의 장점. 워드프레스가 인기.","keyword":"워드프레스"}'
```

```json
{
  "count": 3,
  "matches": ["워드프레스", "워드프레스", "워드프레스"],
  "engine": "lindera"
}
```

## Build

```bash
cargo build --release
```

First build downloads + compiles mecab-ko-dic (~2-3 minutes on a workstation, 5-10 on a small VM). Resulting binary is ~120 MB because the dictionary ships embedded — no runtime dict file to manage.

## Run

```bash
cargo run --release
# or with custom bind
BIND=0.0.0.0:8787 cargo run --release
```

## Deploy

Single static binary, no runtime dependencies. Drop on any Linux host with `glibc`. The `deploy/seo-for-korean-gateway.service` systemd unit is hardened (`PrivateTmp`, `ProtectSystem=strict`, etc).

A multi-stage `Dockerfile` is included (Debian trixie-slim runtime).

## Why a separate service

The PHP plugin runs inside whatever WP host the user picked — shared hosting, missing PHP extensions, no Korean morphology stack. Pushing analysis to a single shared service means:

- One ko-dic install serves every WP site
- Plugin stays small (single zip, no native deps)
- Engine can be upgraded independently of every WP install
- Plugin falls back gracefully (in-PHP regex) when the gateway is unreachable

## License

GPL-2.0-or-later. Same as the plugin.
