use anyhow::Result;
use serde::Deserialize;
use thiserror::Error;

use reqwest::Client;

use std::fmt::Write;

use lazy_static::lazy_static;
use regex::Regex;

use teloxide::{
    prelude::*,
    types::{MediaKind, MediaText, MessageKind, ParseMode},
};

const USER_AGENT: &str =
    "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/107.0.0.0 Safari/537.36 Edg/107.0.1418.24";

const POPULAR_UP: &[u64] = &[431073645, 635041, 1773346];

#[derive(Debug, Error)]
enum FetchError {
    #[error("code is not zero")]
    NonZeroCode,
}

#[tokio::main]
async fn main() -> Result<()> {
    pretty_env_logger::init();
    log::info!("Starting workarounds bot...");
    let bot = Bot::from_env();

    teloxide::repl(bot, |bot: Bot, m: Message| async move {
        let client = reqwest::ClientBuilder::new()
            .user_agent(USER_AGENT)
            .build()?;

        let mut resp_text = String::new();

        if let MessageKind::Common(msg) = m.kind {
            if msg.forward.is_some() || msg.reply_to_message.is_some() {
                return Ok(());
            }

            if let MediaKind::Text(MediaText { text, .. }) = msg.media_kind {
                if let Some(name) = filter_query(&text) {
                    if let Err(_) = req_client_init(&client).await {
                        log::error!("bilibili initialize failed");
                        return Ok(());
                    }

                    if name.is_empty(){
                        return Ok(());
                    }

                    for &mid in POPULAR_UP {
                        for (avid, title) in fetch_videos(&client, mid, name)
                            .await
                            .expect("Error occured in fetching")
                        {
                            resp_text
                                .write_fmt(format_args!(
                                    "<a href=\"https://www.bilibili.com/video/av{avid}\">{title}</a>\n"
                                ))
                                .unwrap();
                        }
                    }
                }
            }
        }

        if !resp_text.is_empty() {

        bot.send_message(m.chat.id, resp_text.trim_end())
            .parse_mode(ParseMode::Html)
            .disable_web_page_preview(true)
            .await?;
        }

        Ok(())
    })
    .await;

    Ok(())
}

async fn req_client_init(client: &Client) -> Result<()> {
    client.get("https://www.bilibili.com").send().await?;
    Ok(())
}

fn filter_query(text: &str) -> Option<&str> {
    lazy_static! {
        static ref RE1: Regex =
            Regex::new("^(.*)(在哪里|在哪|哪里有|哪儿有|哪有|在哪儿)$").unwrap();
        static ref RE2: Regex = Regex::new("^(哪里有|哪儿有|哪有)(.*)$").unwrap();
    }

    RE1.captures(text)
        .and_then(|cap| cap.get(1).map(|m| m.as_str()))
        .or_else(|| {
            RE2.captures(text)
                .and_then(|cap| cap.get(2).map(|m| m.as_str()))
        })
}

async fn fetch_videos(
    client: &Client,
    mid: u64,
    keyword: &str,
) -> Result<impl Iterator<Item = (u64, String)>> {
    let resp = client
        .get("https://api.bilibili.com/x/space/wbi/arc/search")
        .query(&[
            ("ps", "2"),
            ("mid", mid.to_string().as_str()),
            ("keyword", keyword),
        ])
        .send()
        .await?;
    let result: Response = resp.json().await?;
    if result.code != 0 {
        log::error!("error searching video of {mid}: {}", result.message);
        Err(FetchError::NonZeroCode)?;
    }

    Ok(result
        .data
        .list
        .vlist
        .into_iter()
        .map(|VideoInfo { aid, title }| (aid, title)))
}

#[derive(Deserialize)]
struct Response {
    code: i64,
    message: String,
    data: SpaceResponse,
}

#[derive(Deserialize)]
struct SpaceResponse {
    list: SpaceList,
}

#[derive(Deserialize)]
struct SpaceList {
    vlist: Vec<VideoInfo>,
}

#[derive(Deserialize)]
struct VideoInfo {
    aid: u64,
    title: String,
}
