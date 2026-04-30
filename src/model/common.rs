use serde::Serialize;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ArtistRef {
    pub id: String,
    pub name: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct AlbumRef {
    pub id: String,
    pub name: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Thumbnail {
    pub height: u32,
    pub url: String,
    pub width: u32,
}
