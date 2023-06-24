use arrayvec::ArrayString;
use once_cell::sync::OnceCell;
use tokio::sync::Mutex;

use crate::expiry;
use crate::Client;
use moka::future::Cache;

pub async fn get_mixin_key() -> anyhow::Result<ArrayString<32>> {
    let lock = MIXIN_KEY.get_or_init(|| {
        Mutex::new(
            Cache::builder()
                .max_capacity(1)
                .expire_after(expiry::DayExpiry)
                .build_with_hasher(ahash::RandomState::default()),
        )
    });
    let guard = lock.lock().await;
    if let Some(key) = guard.get(&()) {
        return Ok(key);
    }

    let client = req_client_build().await?;
    let wbi_key = bili_wbi_sign_rs::parse_wbi_keys(
        &client
            .get(bili_wbi_sign_rs::WBI_URI)
            .send()
            .await?
            .bytes()
            .await?,
    )?;
    let _mixin_key = unsafe { bili_wbi_sign_rs::mixin_key(wbi_key.as_bytes()) };
    let mut mixin_key = ArrayString::new();
    mixin_key.push_str(&_mixin_key);
    guard.insert((), mixin_key).await;
    Ok(mixin_key)
}

pub async fn req_client_build() -> anyhow::Result<Client> {
    let client = build_reqwest_client()?;
    client.get("https://www.bilibili.com").send().await?;
    Ok(client)
}

fn build_reqwest_client() -> anyhow::Result<Client> {
    use reqwest_middleware::ClientBuilder;
    use reqwest_retry::policies::ExponentialBackoff;
    use reqwest_retry::RetryTransientMiddleware;

    let retry_policy = ExponentialBackoff::builder().build_with_max_retries(5);
    Ok(ClientBuilder::new(
        reqwest::Client::builder()
            .cookie_store(true)
            .user_agent(crate::USER_AGENT)
            .build()?,
    )
    .with(RetryTransientMiddleware::new_with_policy(retry_policy))
    .build())
}

pub static MIXIN_KEY: OnceCell<Mutex<Cache<(), ArrayString<32>, ahash::RandomState>>> =
    OnceCell::new();
