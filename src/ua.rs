use rand::seq::{SliceRandom, IteratorRandom};

pub fn random_user_agent() -> &'static str {
    lazy_static::lazy_static! {
        static ref USER_AGENTS: Vec<String> = vec![
            obfstr::obfstr!("Mozilla/5.0 (Intel Mac OS X 0.24; rv:11.35) (KHTML, like Gecko) OPR/148.95 AppleWebKit/11.35").to_string(),
            obfstr::obfstr!("Mozilla/4.0 (compatible; MSIE 6.0; Windows NT 5.1; SV1; .NET CLR 1.1.4322)").to_string(),
            obfstr::obfstr!("Mozilla/5.0 (X11; Ubuntu; Linux i686; rv:24.0) Gecko/20100101 Firefox/24.0").to_string(),
            obfstr::obfstr!("Mozilla/5.0 (Windows NT 6.1; WOW64; Trident/7.0; rv:11.0) like Gecko").to_string(),
            obfstr::obfstr!("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_6) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/14.0.3 Safari/605.1.15").to_string(),
            obfstr::obfstr!("Mozilla/5.0 (X11; CrOS armv7l 13099.110.0) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/84.0.4147.136 Safari/537.36").to_string(),
            obfstr::obfstr!("Mozilla/5.0 (iPhone; CPU iPhone OS 13_3_1 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/13.0.5 Mobile/15E148 Snapchat/10.77.0.54 (like Safari/604.1)").to_string(),
        ];
    };

    let ua = USER_AGENTS.choose(&mut rand::thread_rng())
        .expect("Unable to choose from vector");

    log::debug!("random ua: {}", ua);
    return ua;
}