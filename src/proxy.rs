use std::net::Ipv4Addr;
use std::time::{Duration, Instant};
use reqwest::StatusCode;
use std::cmp::{Ordering, min};
use std::error::Error;
use crate::utility::BoxError;
use crate::random_user_agent;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ProxyTest {
    pub proxy: (Ipv4Addr, u16),
    pub protocol: SupportedProtocols,
    pub status: StatusCode,
    pub text: String,
    pub time: Instant,
    pub rtt: Duration
}

// `PartialOrd` needs to be implemented as well.
impl PartialOrd for ProxyTest {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        return self.rtt.partial_cmp(&other.rtt);
    }
}

impl Ord for ProxyTest {
    fn cmp(&self, other: &Self) -> Ordering {
        return self.rtt.cmp(&other.rtt);
    }
}

#[derive(Clone, Debug)]
pub struct ProxyManager {
    proxies: Vec<ProxyTest>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SupportedProtocols {
    Http, Socks5
}

impl ToString for SupportedProtocols {
    fn to_string(&self) -> String {
        match self {
            SupportedProtocols::Http => "http".to_string(),
            SupportedProtocols::Socks5 => "socks5".to_string(),
        }
    }
}

impl ProxyManager {
    pub fn new() -> ProxyManager {
        return ProxyManager {
            proxies: Vec::new()
        }
    }

    // pub fn import_tests<Tests>(&mut self, tests: &mut Tests)
    //     where Tests: Iterator{
    //     for test in tests {
    //         self.import_test(test);
    //     }
    // }

    pub fn import_test(&mut self, test: ProxyTest) {
        self.proxies.push(test);
    }

    pub async fn test_proxy(protocol: &SupportedProtocols, proxy: &(Ipv4Addr, u16)) -> Result<ProxyTest, Box<dyn Error + Send + Sync>> {
        let scheme = std::format!("{}://{}:{}", protocol.to_string(), proxy.0, proxy.1);
        let client = reqwest::Client::builder().proxy(
            reqwest::Proxy::all(scheme)?
        ).build()?;

        let before_get = Instant::now();
        let response = client.get(obfstr::obfstr!("https://api.ipify.org/"))
            .header(obfstr::obfstr!("User-Agent"), random_user_agent())
            .header(obfstr::obfstr!("Content-Type"), obfstr::obfstr!("application/x-www-form-urlencoded"))
            .header(obfstr::obfstr!("Accept-Language"), obfstr::obfstr!("en-US,en;q=0.9"))
            .timeout(Duration::from_secs(30))
            .send().await?;

        let status = response.status();
        let text = response.text().await?;
        let rtt = Instant::now().duration_since(before_get);

        let test = ProxyTest{
            proxy: proxy.clone(),
            protocol: protocol.clone(),
            time: Instant::now(),
            status, text, rtt
        };

        #[cfg(feature = "logging")]
        log::debug!("test: {:?}", test);
        return Ok(test);
    }

    pub async fn test_proxies(proxies: &Vec<(Ipv4Addr, u16)>) -> Result<Vec<ProxyTest>, Box<dyn Error + Send + Sync>> {
        #[cfg(feature = "logging")]
        log::info!("testing {} proxies: {:?}", proxies.len(), proxies);
        let proxied_ips: Arc<Mutex<Vec<ProxyTest>>> = Arc::new(Mutex::new(Vec::new()));

        const CONCURRENCY: usize = 20;
        for i in (0..proxies.len()).filter(|x| x % CONCURRENCY == 0) {
            log::debug!("[ProxyManager::test_proxies] {}", i);
            async_scoped::TokioScope::scope_and_block(|s| {
                for j in i..min(i + CONCURRENCY, proxies.len()) {
                    let proxy = proxies[j];
                    let proxied_ips_ref = &proxied_ips;

                    #[cfg(feature = "logging")]
                    log::debug!("[ProxyManager::test_proxies] Spawned task {}", j);
                    s.spawn(async move {
                        tokio::time::sleep(Duration::from_millis((j * 10) as u64)).await;
                        let socks_test= match ProxyManager::test_proxy(&SupportedProtocols::Socks5, &proxy).await {
                            Ok(test) => Some(test),
                            Err(_) => None
                        };

                        let http_test = match ProxyManager::test_proxy(&SupportedProtocols::Http, &proxy).await {
                            Ok(test) => Some(test),
                            Err(_) => None
                        };

                        {
                            if socks_test.is_some() {
                                proxied_ips_ref.lock().await.push(socks_test.unwrap());

                                #[cfg(feature = "logging")]
                                log::debug!("success socks5 proxy {:?}", proxy);
                            }

                            if http_test.is_some() {
                                proxied_ips_ref.lock().await.push(http_test.unwrap());

                                #[cfg(feature = "logging")]
                                log::debug!("success http proxy {:?}", proxy);
                            }
                        }
                    });
                }
            });
        }

        let mut tests = Arc::try_unwrap(proxied_ips).unwrap().into_inner();
        tests.sort();
        return Ok(tests);
    }
}