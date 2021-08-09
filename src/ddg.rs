use reqwest::Url;
use std::str::FromStr;
use std::time::Duration;
use std::net::Ipv4Addr;
use regex::Regex;
use std::error::Error;
use crate::proxy::ProxyManager;
use crate::{random_user_agent, Crawler};
use crate::crawler::parse_basic_proxy_pair;

fn filter_result_urls<F>(text: &str, pred: F) -> Result<Vec<reqwest::Url>, Box<dyn Error>>
    where F: Fn(&Url) -> bool {
    lazy_static::lazy_static! {
        static ref DDG_RESULT_PATTERN: Regex
            = Regex::new(r#"<a rel="nofollow" href="(.+)" class='result-link'>"#).unwrap();
    }

    let mut urls = Vec::new();
    for x in DDG_RESULT_PATTERN.captures_iter(text) {
        let url = Url::from_str(&x[1])?;
        if pred(&url) {
            urls.push(url);
        }
    }

    return Ok(urls);
}

fn parse_result_urls(text: &str) -> Result<Vec<reqwest::Url>, Box<dyn Error>>{
    return filter_result_urls(text, |_| -> bool {true});
}

#[derive(Clone)]
pub struct DDGCrawler {
    web: reqwest::Client,
    proxy_mgr: ProxyManager,
    pub timeout: Duration
}

impl DDGCrawler {
    pub fn new(builder: reqwest::ClientBuilder) -> DDGCrawler {
        log::debug!("DDGCrawler type size: {}", std::mem::size_of::<DDGCrawler>());
        return DDGCrawler{
            web: builder.build().expect(obfstr::obfstr!("Unable to construct reqwest::Client")),
            proxy_mgr: ProxyManager::new(),
            timeout: Duration::from_secs(30)
        }
    }

    pub fn from_proxy(proxy: reqwest::Proxy) -> DDGCrawler {
        return DDGCrawler::new(reqwest::Client::builder()
            .proxy(proxy)
            .user_agent(random_user_agent())
        );
    }

    pub async fn public_ip(&mut self) -> Result<String, Box<dyn Error>> {
        let ip = self.web.get(obfstr::obfstr!("https://api.ipify.org/"))
            .header(obfstr::obfstr!("User-Agent"), random_user_agent())
            .header(obfstr::obfstr!("Content-Type"), obfstr::obfstr!("application/x-www-form-urlencoded"))
            .header(obfstr::obfstr!("Accept-Language"), obfstr::obfstr!("en-US,en;q=0.9"))
            .timeout(self.timeout)
            .send().await?
            .text().await?;

        log::debug!("public_ip = {:?}", ip);
        return Ok(ip);
    }
}

// for sync issues: #[async_trait::async_trait(?Send)]
#[async_trait::async_trait]
impl Crawler for DDGCrawler {
    async fn search(&self, text: &str) -> Result<Vec<reqwest::Url>, Box<dyn Error>> {
        let body = std::format!("q={}", text.replace(" ", "+"));
        log::debug!("body: {}", body);

        let response = self.web.post(obfstr::obfstr!("https://html.duckduckgo.com/lite/"))
            .body(body)
            .header(obfstr::obfstr!("User-Agent"), random_user_agent())
            .header(obfstr::obfstr!("Content-Type"), obfstr::obfstr!("application/x-www-form-urlencoded"))
            .header(obfstr::obfstr!("Accept-Language"), obfstr::obfstr!("en-US,en;q=0.9"))
            .timeout(self.timeout)
            .send().await?;

        log::debug!("response: {:?}", response.status());
        let text = response.text().await?;

        log::debug!("text: {:?}", text);
        return Ok(filter_result_urls(text.as_str(), |url: &Url| -> bool {
            let domain = url.domain();
            return domain.is_some() && !domain.unwrap().contains(obfstr::obfstr!("duckduckgo.com"));
        })?);
    }

    async fn scrape_proxies(&self, url: &Url) -> Result<Vec<(Ipv4Addr, u16)>, Box<dyn Error>> {
        let response = self.web.get(url.as_str())
            .header(obfstr::obfstr!("User-Agent"), random_user_agent())
            .header(obfstr::obfstr!("Content-Type"), obfstr::obfstr!("application/x-www-form-urlencoded"))
            .header(obfstr::obfstr!("Accept-Language"), obfstr::obfstr!("en-US,en;q=0.9"))
            .timeout(self.timeout)
            .send().await?;

        log::debug!("response: {:?}", response.status());
        let proxy_pairs = parse_proxy_pairs(response.text().await?.as_str())?;
        return Ok(proxy_pairs);
    }
}

fn parse_html_proxy_pair(text: &str) -> Vec<(Ipv4Addr, u16)> {
    lazy_static::lazy_static! {
        static ref IP_PORT_HTML_PATTERN: Regex = Regex::new(obfstr::obfstr!(r#"<td>([0-9]+\.[0-9]+\.[0-9]+\.[0-9]+)</td>[\n\r\t ]*<td>([0-9]+)[\n\r\t ]*</td>"#))
            .expect(obfstr::obfstr!("ip_port_html_pattern construction"));
    }

    let mut proxy_pairs: Vec<(Ipv4Addr, u16)> = Vec::new();
    let matches: regex::Matches = IP_PORT_HTML_PATTERN.find_iter(text);
    for m in matches {
        let captures = IP_PORT_HTML_PATTERN.captures(m.as_str())
            .expect(obfstr::obfstr!("Unable to obtain captures for match"));

        let ip_parse = Ipv4Addr::from_str(&captures[1]);
        if ip_parse.is_err() {
            continue;
        }

        let ip = ip_parse.unwrap();
        let port_parse = u16::from_str(&captures[2]);
        if port_parse.is_err() {
            continue;
        }

        let port = port_parse.unwrap();
        if !proxy_pairs.contains(&(ip, port)) {
            proxy_pairs.push((ip, port))
        }
    }

    return proxy_pairs;
}

fn parse_proxy_pairs(text: &str) -> Result<Vec<(Ipv4Addr, u16)>, Box<dyn Error>> {
    let mut proxy_pairs: Vec<(Ipv4Addr, u16)> = Vec::new();
    proxy_pairs.extend(parse_basic_proxy_pair(text));
    proxy_pairs.extend(parse_html_proxy_pair(text));
    return Ok(proxy_pairs);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_proxy_pairs() {
        assert_eq!(
            parse_proxy_pairs("127.0.0.1:8080 and 192.168.1.1:5554").unwrap(),
            [
                (Ipv4Addr::from_str("127.0.0.1").unwrap(), 8080),
                (Ipv4Addr::from_str("192.168.1.1").unwrap(), 5554)
            ]
        );
        assert_eq!(
            parse_proxy_pairs(r#"<td>127.0.0.1</td>
            <td>8080</td>
            <td>95.104.54.227</td>
            <td>42119</td>
            "#).unwrap(),
            [
                (Ipv4Addr::from_str("127.0.0.1").unwrap(), 8080),
                (Ipv4Addr::from_str("95.104.54.227").unwrap(), 42119)
            ]
        );
    }

    #[test]
    fn test_parse_result_urls() -> Result<(), Box<dyn Error>>  {
        let html = r#"
 <tr>
    <td valign="top">4.&nbsp;</td>
    <td>
        <a rel="nofollow" href="https://dfir.gov/2010/03/how-to-find-cheese-diy.html" class='result-link'>Free Horse List - HorseScan</a>
    </td>
  </tr>

  <tr>
    <td>&nbsp;&nbsp;&nbsp;</td>
    <td class='result-snippet'>
      <b>Free</b> <b>Horse</b> <b>List</b>. All the horses are subjected to a detailed check(every 10 minutes) before coming to the <b>list</b>. Each <b>horse</b> is controlled by the parameter set
    </td>
  </tr>

  <tr>
    <td>&nbsp;&nbsp;&nbsp;</td>
    <td>
      <span class='link-text'>www.HorseScan.net</span>
    </td>
  </tr>
  "#;

        let urls = parse_result_urls(html)?;
        assert_eq!(urls, [Url::from_str("https://dfir.gov/2010/03/how-to-find-cheese-diy.html")?]);
        return Ok(());
    }
}