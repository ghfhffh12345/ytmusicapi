# ytmusicapi

Rust workspace for `ytmusicapi`, with:

- `crates/ytmusicapi`: the library crate
- `crates/ytmusicapi-cli`: a small CLI that converts copied browser headers into `browser.json`

## Generate `browser.json` with `ytmusicapi-cli`

This repository uses the local workspace crate `ytmusicapi-cli` to generate `browser.json`.
It does not use the Python project's setup flow.

1. Sign in to YouTube Music in your browser.
2. Open the browser devtools network tab and find a request to `music.youtube.com` such as `POST /youtubei/v1/browse`.
3. Copy the request headers as plain text and save them to `browser.txt`.
   The CLI accepts the raw copied header dump on `stdin`, including the leading `POST ... HTTP/...` request line if your browser includes it.
4. Run the CLI from the workspace root:

```bash
cargo run -p ytmusicapi-cli < browser.txt
```

5. The command writes `browser.json` into the current working directory.
   If you run it from the repo root, the file will be created at `./browser.json`, which matches the path used by the repo's ignored live tests.

The header dump must include at least:

- `Cookie`, with a `__Secure-3PAPISID=...` entry
- `X-Goog-AuthUser`

Example input shape:

```text
POST /youtubei/v1/browse HTTP/3
Host: music.youtube.com
User-Agent: Mozilla/5.0
Content-Type: application/json
X-Goog-AuthUser: 0
X-Origin: https://music.youtube.com
X-Youtube-Client-Name: 67
X-Youtube-Client-Version: 1.20250501.01.00
Cookie: __Secure-3PAPISID=...
```
