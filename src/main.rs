use anyhow::Result;
use serde::Deserialize;
use thiserror::Error;

use reqwest::Client;
use tokio::sync::RwLock;
use ttl_cache::TtlCache;

use std::{collections::HashMap, fmt::Write, sync::Arc};

use lazy_static::lazy_static;
use regex::Regex;

lazy_static! {
    static ref MIXIN_KEY: Arc<RwLock<TtlCache<&'static str, String>>> =
        Arc::new(RwLock::new(TtlCache::new(1)));
}

use teloxide::{
    adaptors::{
        throttle::{Limits, Settings},
        Throttle,
    },
    prelude::*,
    types::{MediaKind, MediaText, MessageKind, ParseMode},
};

use teloxide_core::Bot;

const USER_AGENT: &str =
    "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/107.0.0.0 Safari/537.36 Edg/107.0.1418.24";

const POPULAR_UP: &[u64] = &[431073645, 635041, 1773346];

#[derive(Debug, Error)]
enum FetchError {
    #[error("{0}")]
    NonZeroCode(i64),
}

#[tokio::main]
async fn main() -> Result<()> {
    pretty_env_logger::init();
    log::info!("Starting workarounds bot...");
    let bot = Throttle::spawn_with_settings(
        Bot::from_env(),
        Settings::default()
            .check_slow_mode()
            .no_retry()
            .limits(Limits {
                messages_per_min_chat: 5,
                messages_per_min_channel: 5,
                messages_per_sec_overall: 3,
                messages_per_sec_chat: 1,
            }),
    );

    teloxide::repl(bot, |bot: Throttle<Bot>, m: Message| async move {
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
                    if req_client_init(&client).await.is_err() {
                        log::error!("bilibili initialize failed");
                        return Ok(());
                    }

                    if name.is_empty(){
                        return Ok(());
                    }

                    if name == "老婆" {
                        resp_text += "在诚哥壶里\n";
                    }

                    for &mid in POPULAR_UP {
                        let video_list = fetch_videos(&client, mid, name)
                            .await;
                        match video_list {
                            Ok(list) => {
                                for (avid, title) in list
                                {
                                    resp_text
                                        .write_fmt(format_args!(
                                            "<a href=\"https://www.bilibili.com/video/av{avid}\">{title}</a>\n"
                                        ))
                                        .unwrap();
                                }
                            },
                            Err(e) => resp_text.write_fmt(format_args!("呜呜呜出错了 {}\n", e)).unwrap(),
                        }
                    }
                }
            }
        }

        if !resp_text.is_empty() {

        bot.send_message(m.chat.id, resp_text.trim_end())
            .allow_sending_without_reply(true)
            .disable_web_page_preview(true)
            .parse_mode(ParseMode::Html)
            .reply_to_message_id(m.id)
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
) -> std::result::Result<impl Iterator<Item = (u64, String)>, Box<dyn std::error::Error>> {
    let mut query_map: HashMap<String, String> = HashMap::new();
    query_map.insert("ps".into(), "2".into());
    query_map.insert("mid".into(), mid.to_string());
    query_map.insert("keyword".into(), keyword.into());

    let params;

    let c_mixin_key = MIXIN_KEY.clone();
    if let Some(mixin_key) = c_mixin_key.read().await.get("mixin_key") {
        params = bili_wbi_sign_rs::wbi_sign_encode(query_map, &mixin_key);
    } else {
        let wbi_key = bili_wbi_sign_rs::get_wbi_keys(&client).await?;
        let mixin_key = unsafe { bili_wbi_sign_rs::mixin_key(wbi_key.as_bytes()) };

        params = bili_wbi_sign_rs::wbi_sign_encode(query_map, &mixin_key);

        if let Some(ttl) = bili_wbi_sign_rs::expires_after()
            .map(|d| d.to_std().ok())
            .flatten()
        {
            c_mixin_key.write().await.insert("mixin_key", mixin_key, ttl);
        }
    }

    let resp = client
        .get("https://api.bilibili.com/x/space/wbi/arc/search")
        .query(&params)
        .send()
        .await?;

    let result: Response = resp.json().await?;

    if result.code != 0 {
        log::error!("error searching video of {mid}: {}", result.message);
        Err(FetchError::NonZeroCode(result.code))?;
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
