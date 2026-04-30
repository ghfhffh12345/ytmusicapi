from __future__ import annotations

import json
from dataclasses import dataclass
from pathlib import Path

from ytmusicapi import YTMusic


@dataclass(frozen=True)
class FixtureCase:
    name: str
    query: str
    filter_name: str | None = None
    ignore_spelling: bool = False


FIXTURE_CASES = [
    FixtureCase("default_mixed", "daft punk"),
    # Reference-only anonymous search capture: upstream filtered song search is unstable
    # in this environment, so the parsed result may legitimately remain empty.
    FixtureCase("songs", "ABBA", "songs"),
    # Reference-only anonymous search capture: upstream filtered video search is also
    # unstable here, but this must stay tied to a real anonymous `search()` response.
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


def capture_search_case(client: RecordingYTMusic, case: FixtureCase) -> tuple[object, object]:
    kwargs = {}
    if case.filter_name is not None:
        kwargs["filter"] = case.filter_name
    if case.ignore_spelling:
        kwargs["ignore_spelling"] = True

    parsed = client.search(case.query, **kwargs)
    return client.last_response, parsed


def main() -> None:
    root = Path(__file__).resolve().parents[1] / "tests" / "fixtures" / "search"
    raw_dir = root / "raw"
    expected_dir = root / "expected"
    raw_dir.mkdir(parents=True, exist_ok=True)
    expected_dir.mkdir(parents=True, exist_ok=True)

    client = RecordingYTMusic()

    for case in FIXTURE_CASES:
        raw, parsed = capture_search_case(client, case)

        (raw_dir / f"{case.name}.json").write_text(json.dumps(raw, indent=2, sort_keys=True) + "\n")
        (expected_dir / f"{case.name}.json").write_text(
            json.dumps(parsed, indent=2, sort_keys=True) + "\n"
        )


if __name__ == "__main__":
    main()
