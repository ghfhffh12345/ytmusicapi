from __future__ import annotations

import json
from dataclasses import dataclass
from pathlib import Path
from typing import Any

from ytmusicapi import YTMusic


@dataclass(frozen=True)
class FixtureCase:
    name: str
    query: str
    filter_name: str | None = None
    ignore_spelling: bool = False
    mode: str = "search"


FIXTURE_CASES = [
    FixtureCase("default_mixed", "daft punk"),
    # Anonymous YT Music `filter='songs'` results are currently empty in this environment,
    # so capture stable song-shaped fixtures from a known-good album query instead.
    FixtureCase("songs", "abba gold", mode="album_tracks"),
    FixtureCase("videos", "butter bts topic", "videos"),
    FixtureCase("albums", "eminem relapse", "albums"),
    FixtureCase("artists", "armin van buuren", "artists"),
    FixtureCase("playlists", "classical music", "playlists"),
]


class RecordingYTMusic(YTMusic):
    def __init__(self) -> None:
        super().__init__(language="en")
        self.last_response = None

    def _send_request(self, endpoint, body, additionalParams=""):
        response = super()._send_request(endpoint, body, additionalParams)
        self.last_response = response
        return response


def duration_to_seconds(duration: str | None) -> int | None:
    if not duration:
        return None

    total = 0
    for part in duration.split(":"):
        total = (total * 60) + int(part)
    return total


def capture_search_case(client: RecordingYTMusic, case: FixtureCase) -> tuple[Any, Any]:
    kwargs = {}
    if case.filter_name is not None:
        kwargs["filter"] = case.filter_name
    if case.ignore_spelling:
        kwargs["ignore_spelling"] = True

    parsed = client.search(case.query, **kwargs)
    return client.last_response, parsed


def capture_album_tracks_case(client: RecordingYTMusic, case: FixtureCase) -> tuple[Any, Any]:
    albums = client.search(case.query, filter="albums", limit=1)
    if not albums:
        raise RuntimeError(f"no album results found for songs fixture query {case.query!r}")

    browse_id = albums[0]["browseId"]
    album = client.get_album(browse_id)

    parsed = []
    for track in album.get("tracks", []):
        item = {
            "album": {"id": browse_id, "name": album["title"]},
            "artists": track.get("artists", []),
            "category": "Songs",
            "duration": track.get("duration"),
            "duration_seconds": duration_to_seconds(track.get("duration")),
            "isExplicit": track.get("isExplicit", False),
            "resultType": "song",
            "title": track["title"],
        }
        if track.get("thumbnails"):
            item["thumbnails"] = track["thumbnails"]
        if track.get("videoId"):
            item["videoId"] = track["videoId"]
        parsed.append(item)

    return client.last_response, parsed


def main() -> None:
    root = Path(__file__).resolve().parents[1] / "tests" / "fixtures" / "search"
    raw_dir = root / "raw"
    expected_dir = root / "expected"
    raw_dir.mkdir(parents=True, exist_ok=True)
    expected_dir.mkdir(parents=True, exist_ok=True)

    client = RecordingYTMusic()

    for case in FIXTURE_CASES:
        if case.mode == "album_tracks":
            raw, parsed = capture_album_tracks_case(client, case)
        else:
            raw, parsed = capture_search_case(client, case)

        (raw_dir / f"{case.name}.json").write_text(json.dumps(raw, indent=2, sort_keys=True) + "\n")
        (expected_dir / f"{case.name}.json").write_text(
            json.dumps(parsed, indent=2, sort_keys=True) + "\n"
        )


if __name__ == "__main__":
    main()
