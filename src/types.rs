use serde::Deserialize;

#[derive(Deserialize)]
pub struct Response {
    pub code: i64,
    pub message: String,
    pub data: SpaceResponse,
}

#[derive(Deserialize)]
pub struct SpaceResponse {
    pub list: SpaceList,
}

#[derive(Deserialize)]
pub struct SpaceList {
    pub vlist: Vec<VideoInfo>,
}

#[derive(Deserialize)]
pub struct VideoInfo {
    pub aid: u64,
    pub title: String,
}
