mod ddg;
mod ua;
mod utility;
pub mod crawler;
pub mod proxy;

pub use ua::random_user_agent;
pub use crawler::public_ip;
pub use crawler::Crawler;
pub use ddg::DDGCrawler;