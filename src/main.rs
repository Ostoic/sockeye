use std::error::Error;
use env_logger::Env;
use sockeye::proxy::ProxyManager;
use sockeye::Crawler;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>>  {
    let env = Env::default()
        .filter_or("MY_LOG_LEVEL", "debug")
        .write_style_or("MY_LOG_STYLE", "always");

    env_logger::init_from_env(env);
    log::info!("Hello!");

    let crawler
        // = sockeye::DDGCrawler::new(reqwest::Client::builder());
        = sockeye::DDGCrawler::from_proxy(reqwest::Proxy::all("socks5://10.179.205.104:9050").unwrap());

    let proxies = match crawler.crawl("free proxy list", 10).await {
        Ok(proxies) => Option::Some(proxies),
        Err(_) => None
    };

    if proxies.is_none() {
        log::error!("Crawler found no proxies: {:?}", proxies);
        return Ok(())
    }

    log::info!("Crawler found proxies: {:?}", proxies);
    let mut mgr = ProxyManager::new();
    match ProxyManager::test_proxies(&proxies.unwrap()).await {
        Ok(proxied_ips) => {
            log::info!("Usable proxies: {:?}", proxied_ips);
            for proxy in proxied_ips {
                mgr.import_test(proxy);
            }
        },
        Err(e) => {
            log::error!("Error: {:?}", e);
        }
    }

    log::info!("Public IP: {:?}", sockeye::crawler::public_ip().await);
    return Ok(())
}