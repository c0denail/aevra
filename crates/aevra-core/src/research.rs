use crate::{
    types::{now, Source},
    Error, Result,
};
use reqwest::{redirect::Policy, Client};
use scraper::{Html, Selector};
use std::{
    collections::HashSet,
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
    time::Duration,
};
use url::Url;

const SEARCH_URL: &str = "https://html.duckduckgo.com/html/";
const MAX_BYTES: usize = 1_000_000;

pub fn validate_url(raw: &str) -> Result<Url> {
    let url = Url::parse(raw).map_err(|_| Error::Network("Geçersiz web adresi.".into()))?;
    if url.scheme() != "https"
        || !url.username().is_empty()
        || url.password().is_some()
        || url.port().is_some_and(|p| p != 443)
        || raw.len() > 2048
    {
        return Err(Error::Network(
            "Yalnızca kimlik bilgisi içermeyen standart HTTPS adreslerine izin verilir.".into(),
        ));
    }
    let host = url
        .host_str()
        .ok_or_else(|| Error::Network("Web adresinin alan adı eksik.".into()))?
        .trim_end_matches('.')
        .to_ascii_lowercase();
    if !host.contains('.')
        || host.parse::<IpAddr>().is_ok()
        || host.contains(':')
        || [
            "localhost",
            ".localhost",
            ".local",
            ".internal",
            ".lan",
            ".home",
            ".test",
            ".invalid",
            ".arpa",
        ]
        .iter()
        .any(|s| host == *s || host.ends_with(s))
    {
        return Err(Error::Network(
            "Yerel ağ ve IP adreslerine web erişimi kapalı.".into(),
        ));
    }
    Ok(url)
}

pub fn public_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => public_v4(ip),
        IpAddr::V6(ip) => public_v6(ip),
    }
}

fn public_v4(ip: Ipv4Addr) -> bool {
    let [a, b, c, _] = ip.octets();
    !(a == 0
        || a == 10
        || a == 127
        || a >= 224
        || (a == 100 && (64..=127).contains(&b))
        || (a == 169 && b == 254)
        || (a == 172 && (16..=31).contains(&b))
        || (a == 192 && (b == 168 || b == 0 || (b == 88 && c == 99)))
        || (a == 198 && (b == 18 || b == 19 || (b == 51 && c == 100)))
        || (a == 203 && b == 0 && c == 113))
}

fn public_v6(ip: Ipv6Addr) -> bool {
    let s = ip.segments();
    // Only globally routed unicast; reject special-purpose, documentation and 6to4.
    (s[0] & 0xe000) == 0x2000
        && !(s[0] == 0x2001 && (s[1] < 0x0200 || s[1] == 0x0db8))
        && s[0] != 0x2002
        && s[0] != 0x3fff
}

/// Each hop gets a freshly validated DNS answer pinned into the HTTP connector.
/// No proxies, cookies, JavaScript, downloads, or automatic redirects.
async fn fetch_public(raw: &str) -> Result<String> {
    let mut url = validate_url(raw)?;
    for _ in 0..4 {
        let host = url.host_str().unwrap().to_string();
        let addresses = tokio::net::lookup_host((host.as_str(), 443))
            .await
            .map_err(|_| Error::Network("Alan adı çözümlenemedi.".into()))?
            .collect::<Vec<_>>();
        if addresses.is_empty() || addresses.iter().any(|a| !public_ip(a.ip())) {
            return Err(Error::Network(
                "Güvenli olmayan ağ adresi engellendi.".into(),
            ));
        }
        let client = Client::builder()
            .https_only(true)
            .no_proxy()
            .redirect(Policy::none())
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(12))
            .user_agent("Aevra/0.1 (user-initiated research)")
            .resolve_to_addrs(&host, &addresses)
            .build()
            .map_err(|_| Error::Network("Web bağlantısı hazırlanamadı.".into()))?;
        let mut response = client
            .get(url.clone())
            .send()
            .await
            .map_err(|_| Error::Network("Web kaynağına ulaşılamadı.".into()))?;
        if response.status().is_redirection() {
            let target = response
                .headers()
                .get(reqwest::header::LOCATION)
                .and_then(|h| h.to_str().ok())
                .ok_or_else(|| Error::Network("Geçersiz yönlendirme.".into()))?;
            url = validate_url(
                url.join(target)
                    .map_err(|_| Error::Network("Geçersiz yönlendirme adresi.".into()))?
                    .as_str(),
            )?;
            continue;
        }
        if !response.status().is_success() {
            return Err(Error::Network(format!(
                "Web hizmeti yanıt vermedi ({}).",
                response.status()
            )));
        }
        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        if !content_type.starts_with("text/html") && !content_type.starts_with("text/plain") {
            return Err(Error::Network(
                "Yalnızca HTML veya düz metin kaynakları okunur.".into(),
            ));
        }
        if response
            .content_length()
            .is_some_and(|l| l > MAX_BYTES as u64)
        {
            return Err(Error::Network("Web sayfası boyut sınırını aşıyor.".into()));
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response
            .chunk()
            .await
            .map_err(|_| Error::Network("Web kaynağı okunamadı.".into()))?
        {
            if bytes.len() + chunk.len() > MAX_BYTES {
                return Err(Error::Network("Web sayfası boyut sınırını aşıyor.".into()));
            }
            bytes.extend_from_slice(&chunk);
        }
        return Ok(String::from_utf8_lossy(&bytes).into_owned());
    }
    Err(Error::Network(
        "Web sayfası çok fazla yönlendirme yaptı.".into(),
    ))
}

pub async fn research(query: &str) -> Result<Vec<Source>> {
    research_with_limit(query, 3).await
}

pub async fn research_with_limit(query: &str, limit: usize) -> Result<Vec<Source>> {
    let mut url = Url::parse(SEARCH_URL).unwrap();
    url.query_pairs_mut().append_pair("q", query);
    let html = fetch_public(url.as_str()).await?;
    let candidates = parse_results(&html);
    if candidates.is_empty() {
        return Err(Error::Network(
            "Arama hizmeti sonuç vermedi veya erişimi sınırlandırdı. Daha sonra tekrar deneyin."
                .into(),
        ));
    }
    let mut sources = Vec::new();
    for mut candidate in candidates.into_iter().take(limit.clamp(1, 5)) {
        candidate.id = sources.len() + 1;
        let excerpt = match fetch_public(&candidate.url).await {
            Ok(page) => extract_text(&page),
            Err(_) => String::new(),
        };
        candidate.excerpt = if excerpt.chars().count() >= 80 {
            excerpt.chars().take(1300).collect()
        } else {
            format!(
                "Arama sonucu özeti; sayfa içeriği alınamadı: {}",
                candidate.excerpt
            )
        };
        sources.push(candidate);
    }
    Ok(sources)
}

fn parse_results(html: &str) -> Vec<Source> {
    let document = Html::parse_document(html);
    let result = Selector::parse(".result").unwrap();
    let link = Selector::parse("a.result__a").unwrap();
    let snippet = Selector::parse(".result__snippet").unwrap();
    let mut seen = HashSet::new();
    document
        .select(&result)
        .filter_map(|item| {
            let anchor = item.select(&link).next()?;
            let href = anchor.value().attr("href")?;
            let absolute = if href.starts_with("//") {
                format!("https:{href}")
            } else {
                href.to_owned()
            };
            let parsed = Url::parse(&absolute).ok()?;
            let target = if parsed
                .host_str()
                .is_some_and(|h| h == "duckduckgo.com" || h.ends_with(".duckduckgo.com"))
            {
                parsed
                    .query_pairs()
                    .find(|(k, _)| k == "uddg")?
                    .1
                    .to_string()
            } else {
                absolute
            };
            let mut target = validate_url(&target).ok()?;
            target.set_fragment(None);
            if !seen.insert(target.to_string()) {
                return None;
            }
            let title = anchor
                .text()
                .collect::<Vec<_>>()
                .join(" ")
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ");
            let excerpt = item
                .select(&snippet)
                .next()?
                .text()
                .collect::<Vec<_>>()
                .join(" ");
            if excerpt.trim().is_empty() {
                return None;
            }
            Some(Source {
                id: 0,
                title: title.chars().take(200).collect(),
                url: target.into(),
                excerpt: excerpt.chars().take(700).collect(),
                accessed_at: now(),
            })
        })
        .take(6)
        .collect()
}

fn extract_text(html: &str) -> String {
    let document = Html::parse_document(html);
    let selector = Selector::parse("title, main p, article p").unwrap();
    let pieces = document
        .select(&selector)
        .map(|node| node.text().collect::<Vec<_>>().join(" "))
        .collect::<Vec<_>>();
    pieces
        .join(" ")
        .split_whitespace()
        .take(700)
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_local_networks_and_non_https_urls() {
        for url in [
            "http://example.com",
            "https://127.0.0.1/",
            "https://[::1]/",
            "https://server.local/",
            "https://localhost/",
            "https://a:b@example.com/",
            "https://example.com:8443/",
            "file:///C:/secret",
        ] {
            assert!(validate_url(url).is_err(), "{url}");
        }
        assert!(validate_url("https://www.rust-lang.org/learn").is_ok());
        for ip in [
            "0.0.0.0",
            "10.1.2.3",
            "127.0.0.1",
            "169.254.169.254",
            "100.64.1.1",
            "172.16.1.1",
            "192.168.1.1",
            "198.19.0.1",
            "192.0.2.1",
            "224.0.0.1",
            "::1",
            "::ffff:127.0.0.1",
            "fc00::1",
            "2001:db8::1",
            "2002:7f00:1::",
        ] {
            assert!(!public_ip(ip.parse().unwrap()), "{ip}");
        }
        assert!(public_ip("1.1.1.1".parse().unwrap()));
        assert!(public_ip("2606:4700:4700::1111".parse().unwrap()));
    }
    #[test]
    fn extracts_real_search_sources_and_rejects_unsafe_results() {
        let html = r#"<div class="result"><a class="result__a" href="//duckduckgo.com/l/?uddg=https%3A%2F%2Fexample.com%2F">Örnek</a><a class="result__snippet">Gerçek bir özet.</a></div><div class="result"><a class="result__a" href="https://127.0.0.1/">Unsafe</a><a class="result__snippet">secret</a></div>"#;
        let sources = parse_results(html);
        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].url, "https://example.com/");
    }
}
