use anyhow::Result;
use bili_wbi_sign_rs::wbi_sign_encode;
use thiserror::Error;

use std::{collections::HashMap, fmt::Write};

use regex::Regex;

use teloxide::{
    adaptors::{
        throttle::{Limits, Settings},
        Throttle,
    },
    prelude::*,
    types::{MediaKind, MediaText, MessageKind, ParseMode},
};
use teloxide_core::{types::MessageCommon, Bot};

use yuanshen_workarounds_bot::{types, utils, Client};

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
        let client = utils::req_client_build().await;
        let Ok(client) = client else {
            log::error!("http client initialize failed");
            return Ok(());
        };

        let mut resp_text = String::new();

        match m.kind {
            MessageKind::Common(MessageCommon{media_kind: MediaKind::Text(MediaText{text, ..}), ..})
                if m.forward().is_none() && m.reply_to_message().is_none() => {
            let Some(name) = filter_query(&text) else {
                return Ok(());
            }; 

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
                    Err(e) => resp_text.write_fmt(format_args!("呜呜呜出错了 {e}\n")).unwrap(),
                }
            }
            }
            _ => (),
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

fn filter_query(text: &str) -> Option<&str> {
    lazy_static::lazy_static! {
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
) -> anyhow::Result<impl Iterator<Item = (u64, String)>> {
    let mut query_map: HashMap<String, String> = HashMap::new();
    query_map.insert("ps".into(), "2".into());
    query_map.insert("mid".into(), mid.to_string());
    query_map.insert("keyword".into(), keyword.into());

    let mixin_key = utils::get_mixin_key().await?;
    let params = wbi_sign_encode(query_map, &mixin_key);

    let resp = client
        .get("https://api.bilibili.com/x/space/wbi/arc/search")
        .query(&params)
        .send()
        .await?;

    let result: types::Response = resp.json().await?;

    if result.code != 0 {
        log::error!("error searching video of {mid}: {}", result.message);
        Err(FetchError::NonZeroCode(result.code))?;
    }

    Ok(result
        .data
        .list
        .vlist
        .into_iter()
        .map(|types::VideoInfo { aid, title }| (aid, title)))
}
