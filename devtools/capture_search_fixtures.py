from __future__ import annotations

import json
from pathlib import Path

from ytmusicapi import YTMusic


FIXTURE_CASES = [
    ("default_mixed", "oasis wonderwall", None, False),
    ("songs", "hip hop playlist", "songs", False),
    ("videos", "hip hop playlist", "videos", False),
    ("albums", "eminem relapse", "albums", False),
    ("artists", "armen van buren", "artists", True),
    ("playlists", "classical music", "playlists", False),
]


class RecordingYTMusic(YTMusic):
    def __init__(self) -> None:
        super().__init__()
        self.last_response = None

    def _send_request(self, endpoint, body, additionalParams=""):
        response = super()._send_request(endpoint, body, additionalParams)
        self.last_response = response
        return response


def main() -> None:
    root = Path(__file__).resolve().parents[1] / "tests" / "fixtures" / "search"
    raw_dir = root / "raw"
    expected_dir = root / "expected"
    raw_dir.mkdir(parents=True, exist_ok=True)
    expected_dir.mkdir(parents=True, exist_ok=True)

    client = RecordingYTMusic()

    for name, query, filter_name, ignore_spelling in FIXTURE_CASES:
        kwargs = {}
        if filter_name is not None:
            kwargs["filter"] = filter_name
        if ignore_spelling:
            kwargs["ignore_spelling"] = True

        parsed = client.search(query, **kwargs)
        raw = client.last_response

        (raw_dir / f"{name}.json").write_text(json.dumps(raw, indent=2, sort_keys=True) + "\n")
        (expected_dir / f"{name}.json").write_text(
            json.dumps(parsed, indent=2, sort_keys=True) + "\n"
        )


if __name__ == "__main__":
    main()
