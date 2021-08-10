use reqwest::{Url, StatusCode};
use std::net::Ipv4Addr;
use std::time::Duration;
use std::str::FromStr;
use regex::Regex;
use crate::random_user_agent;
use std::error::Error;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct SearchResult {
    pub urls: Vec<Url>,
}

impl SearchResult {
    pub fn pages(&self) -> SearchResultPages {
        return SearchResultPages {
            index: 0
        }
    }
}

pub struct SearchResultPages {
    pub index: u16
}

impl Iterator for SearchResultPages {
    type Item = SearchResult;

    fn next(&mut self) -> Option<Self::Item> {
        Option::None
        // calls search_n somehow and returns next SearchResult
    }
}

#[async_trait::async_trait]
pub trait SearchEngine {
    async fn search_n(&mut self, text: &str, page: u16) -> Result<SearchResult, Box<dyn Error + Send + Sync>>;
    async fn search(&mut self, text: &str) -> Result<SearchResult, Box<dyn Error + Send + Sync>>;
    async fn scrape(&mut self, url: &Url) -> Result<Vec<(Ipv4Addr, u16)>, Box<dyn Error + Send + Sync>>;
}

// for sync issues: #[async_trait::async_trait(?Send)]
#[async_trait::async_trait]
pub trait Crawler {
    async fn search(&self, text: &str) -> Result<Vec<Url>, Box<dyn Error + Send + Sync>>;
    async fn scrape_proxies(&self, url: &Url) -> Result<Vec<(Ipv4Addr, u16)>, Box<dyn Error + Send + Sync>>;

    async fn crawl(&self, search_term: &str, limit: usize) -> Result<Vec<(Ipv4Addr, u16)>, Box<dyn Error + Send + Sync>>  {
        #[cfg(feature = "logging")]
        log::debug!("Crawler starting: {}", search_term);

        let guarded_proxies: Arc<Mutex<Vec<(Ipv4Addr, u16)>>> = Arc::new(Mutex::new(Vec::new()));
        let urls = self.search(search_term).await?;

        const CONCURRENCY: usize = 20;
        for i in (0..urls.len()).filter(|x| x % CONCURRENCY == 0) {
            {
                let proxies = guarded_proxies.lock().await;
                if proxies.len() > limit {
                    break
                }
            }

            async_scoped::TokioScope::scope_and_block(|s| {
                for j in i..(i + CONCURRENCY) % urls.len() {
                    let guarded_proxies_ref = &guarded_proxies;
                    let url = urls[j].clone();
                    s.spawn(async move {
                        tokio::time::sleep(Duration::from_millis((j * 10) as u64)).await;
                        #[cfg(feature = "logging")]
                        log::debug!("Crawling url {}", url);

                        let new_proxies = match self.scrape_proxies(&url).await {
                            Ok(list) => {
                                if !list.is_empty() {Option::Some(list)}
                                else {None}
                            },
                            Err(_) => None
                        };

                        if new_proxies.is_none() {
                            return;
                        }

                        #[cfg(feature = "logging")]
                        log::debug!("New proxies found: {:?}", new_proxies);

                        let mut proxies = guarded_proxies_ref.lock().await;
                        proxies.extend(new_proxies.unwrap());
                    });
                }
            });
        };

        return Ok(Arc::try_unwrap(guarded_proxies).unwrap().into_inner());
    }

}

pub fn parse_basic_proxy_pair(text: &str) -> Vec<(Ipv4Addr, u16)> {
    lazy_static::lazy_static! {
        static ref IP_PORT_PATTERN: Regex = Regex::new(obfstr::obfstr!(r#"([0-9]+\.[0-9]+\.[0-9]+\.[0-9]+):([0-9]+)"#))
            .expect(obfstr::obfstr!("ip_port_pattern construction"));
    }

    let mut proxy_pairs: Vec<(Ipv4Addr, u16)> = Vec::new();
    let matches: regex::Matches = IP_PORT_PATTERN.find_iter(text);
    for m in matches {
        let split = m.as_str().split(":").collect::<Vec<&str>>();
        let ip = match Ipv4Addr::from_str(split[0]) {
            Ok(ip) => ip,
            Err(_) => {continue}
        };

        let port = match split[1].parse::<u16>() {
            Ok(port) => port,
            Err(_) => {continue}
        };

        if !proxy_pairs.contains(&(ip, port)) {
            proxy_pairs.push((ip, port))
        }
    }

    return proxy_pairs;
}

pub async fn public_ip() -> Result<(StatusCode, String), Box<dyn Error + Send + Sync>> {
    return public_ip_from(&reqwest::ClientBuilder::new().build()?).await;
}

pub async fn public_ip_from(builder: &reqwest::Client) -> Result<(StatusCode, String), Box<dyn Error + Send + Sync>> {
    let response = builder
        .get(obfstr::obfstr!("https://api.ipify.org/"))
        .header(obfstr::obfstr!("User-Agent"), random_user_agent())
        .header(obfstr::obfstr!("Content-Type"), obfstr::obfstr!("application/x-www-form-urlencoded"))
        .header(obfstr::obfstr!("Accept-Language"), obfstr::obfstr!("en-US,en;q=0.9"))
        .timeout(Duration::from_secs(30))
        .send().await?;

    #[cfg(feature = "logging")]
    log::debug!("Client size: {}", std::mem::size_of::<reqwest::Client>());

    #[cfg(feature = "logging")]
    log::debug!("ClientBuilder size: {}", std::mem::size_of::<reqwest::ClientBuilder>());
    // if response.status() != StatusCode::from_u16(200) {
    //     return Err()
    // }

    let status = response.status();
    let ip = response.text().await?;
    return Ok((status, ip));
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;

    #[test]
    fn test_parse_basic_proxy_pair() -> Result<(), Box<dyn Error + Send + Sync>> {
        let test_string = r#"127.0.0.1:8080
        192.168.1.1:1234"#;
        assert_eq!(
            parse_basic_proxy_pair(test_string),
            [
                (Ipv4Addr::from_str("127.0.0.1")?, 8080),
                (Ipv4Addr::from_str("192.168.1.1")?, 1234)
            ]
        );

        return Ok(());
    }
}