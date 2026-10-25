//! The seven environment variables, all optional, all read once before serving
//! (spec #33). A default install cannot be misconfigured: with nothing set,
//! Kamosu binds `0.0.0.0:5266`, stores everything under `/data`, logs at
//! `info`, is nobody's Demo, and sends nothing anywhere.
//!
//! Port 5266 is deliberate (2026-08-26): it spells KAMO on a phone keypad, is
//! unassigned by IANA, sits outside Linux's ephemeral range, and avoids the
//! recipe-app neighbourhood (8080 Tandoor, 9000 Mealie).

use std::net::IpAddr;
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct Config {
    /// Address to bind (`KAMOSU_ADDRESS`, default `0.0.0.0` so dev is reachable
    /// across the LAN).
    pub address: IpAddr,
    /// Port to listen on (`KAMOSU_PORT`, default 5266).
    pub port: u16,
    /// The one durable directory (`KAMOSU_DATA_DIR`, default `/data`). Inside the
    /// container the path is fixed and nobody using Kamosu ever types it; outside a
    /// container — the developer's machine — this override keeps `/data` from being created.
    pub data_dir: PathBuf,
    /// Log filter for `tracing_subscriber` (`KAMOSU_LOG`, default `info`).
    pub log_filter: String,
    /// Whether this instance is a public Demo (ADR 0052): anyone arrives
    /// signed in, and only cooking and shopping save. Named for what it costs
    /// (`KAMOSU_DEMO_ERASES_LIBRARY_DAILY`), since a Demo puts its library
    /// back every day (#404), and on only when the variable says `true`: a
    /// typo leaves a library alone rather than erasing it.
    pub demo: bool,
    /// What a Demo counts its visits with (ADR 0052, #408): nothing, an
    /// Umami, or two settings Kamosu could not use, said once at startup.
    /// Read only in a Demo, so an instance that is none sends nothing
    /// anywhere, whatever is set.
    pub analytics: Result<Option<Analytics>, String>,
}

impl Config {
    pub fn from_env() -> Self {
        Config::from_lookup(|key| std::env::var(key).ok())
    }

    /// The settings as `lookup` gives each variable, so a test can hand them
    /// over without changing the process's own environment.
    pub fn from_lookup(lookup: impl Fn(&str) -> Option<String>) -> Self {
        let text = |key: &str, default: &str| match lookup(key) {
            Some(v) if !v.trim().is_empty() => v,
            _ => default.to_string(),
        };
        let demo = text(DEMO, "false").trim().eq_ignore_ascii_case("true");
        Config {
            address: text("KAMOSU_ADDRESS", "0.0.0.0")
                .trim()
                .parse()
                .unwrap_or_else(|_| "0.0.0.0".parse().expect("valid default address")),
            port: text("KAMOSU_PORT", "5266").trim().parse().unwrap_or(5266),
            data_dir: PathBuf::from(text("KAMOSU_DATA_DIR", "/data")),
            log_filter: text("KAMOSU_LOG", "info"),
            demo,
            analytics: if demo {
                Analytics::from_settings(
                    Some(text(UMAMI_SCRIPT, "")).filter(|v| !v.is_empty()),
                    Some(text(UMAMI_WEBSITE_ID, "")).filter(|v| !v.is_empty()),
                )
            } else {
                Ok(None)
            },
        }
    }

    pub fn bind_address(&self) -> String {
        format!("{}:{}", self.address, self.port)
    }
}

/// The variable that turns an instance into a Demo.
pub const DEMO: &str = "KAMOSU_DEMO_ERASES_LIBRARY_DAILY";

/// The address of a Demo's Umami script, such as
/// `https://umami.example.com/script.js`.
pub const UMAMI_SCRIPT: &str = "KAMOSU_DEMO_UMAMI_SCRIPT";

/// The id Umami gave the Demo's website.
pub const UMAMI_WEBSITE_ID: &str = "KAMOSU_DEMO_UMAMI_WEBSITE_ID";

/// The Umami a Demo counts its visits with (#408). The browser loads the
/// script and sends its counts to the script's own origin, which is the one
/// address the Demo's Content-Security-Policy lets through. Umami sets no
/// cookie, so a Demo asks nobody's consent for it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Analytics {
    /// The script's whole address, as the interface loads it.
    pub script: String,
    /// The website id the script is handed in `data-website-id`.
    pub website_id: String,
    /// `scheme://host[:port]` of the script, and nothing else of it: the
    /// one source the policy names.
    pub origin: String,
}

impl Analytics {
    /// Both settings or neither: one alone counts nothing, and says why.
    fn from_settings(
        script: Option<String>,
        website_id: Option<String>,
    ) -> Result<Option<Analytics>, String> {
        let (script, website_id) = match (script, website_id) {
            (None, None) => return Ok(None),
            (Some(script), Some(id)) => (script.trim().to_string(), id.trim().to_string()),
            (Some(_), None) => {
                return Err(format!("{UMAMI_SCRIPT} is set without {UMAMI_WEBSITE_ID}"));
            }
            (None, Some(_)) => {
                return Err(format!("{UMAMI_WEBSITE_ID} is set without {UMAMI_SCRIPT}"));
            }
        };
        let address = url::Url::parse(&script)
            .ok()
            .filter(|url| matches!(url.scheme(), "https" | "http"))
            .filter(|url| url.username().is_empty() && url.password().is_none())
            // A name or an IPv4 address, and nothing a policy reads as more:
            // `*` there would let every site through, and a `;` would start a
            // directive of its own. A policy cannot name an IPv6 address.
            .filter(|url| match url.host() {
                Some(url::Host::Domain(name)) => name
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '.'),
                Some(url::Host::Ipv4(_)) => true,
                Some(url::Host::Ipv6(_)) | None => false,
            })
            .ok_or_else(|| {
                format!(
                    "{UMAMI_SCRIPT} is not an http(s) address a security policy can name: {script}"
                )
            })?;
        // An Umami website id is a UUID. Letters, digits and dashes are all
        // the interface ever writes into the page.
        if website_id.len() > 64
            || !website_id
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-')
        {
            return Err(format!(
                "{UMAMI_WEBSITE_ID} is not an Umami website id: {website_id}"
            ));
        }
        Ok(Some(Analytics {
            origin: address.origin().ascii_serialization(),
            script: address.to_string(),
            website_id,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn config(settings: &[(&str, &str)]) -> Config {
        let settings: HashMap<String, String> = settings
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        Config::from_lookup(|key| settings.get(key).cloned())
    }

    const SCRIPT: (&str, &str) = (UMAMI_SCRIPT, "https://umami.example.com/script.js");
    const ID: (&str, &str) = (UMAMI_WEBSITE_ID, "94db1cb1-74f4-4a40-ad6c-962362670409");

    #[test]
    fn nothing_set_is_the_default_install() {
        let config = config(&[]);
        assert_eq!(config.bind_address(), "0.0.0.0:5266");
        assert_eq!(config.data_dir, PathBuf::from("/data"));
        assert_eq!(config.log_filter, "info");
        assert!(!config.demo);
        assert_eq!(config.analytics, Ok(None));
    }

    #[test]
    fn an_instance_that_is_no_demo_counts_nothing_whatever_is_set() {
        assert_eq!(config(&[SCRIPT, ID]).analytics, Ok(None));
        assert_eq!(config(&[(DEMO, "yes"), SCRIPT, ID]).analytics, Ok(None));
    }

    #[test]
    fn a_demo_counts_with_the_umami_it_is_given() {
        let counting = config(&[(DEMO, "true"), SCRIPT, ID]);
        assert_eq!(
            counting.analytics,
            Ok(Some(Analytics {
                script: "https://umami.example.com/script.js".into(),
                website_id: "94db1cb1-74f4-4a40-ad6c-962362670409".into(),
                origin: "https://umami.example.com".into(),
            }))
        );
        assert_eq!(config(&[(DEMO, "true")]).analytics, Ok(None));
    }

    #[test]
    fn the_origin_keeps_a_port_and_drops_the_rest() {
        let config = config(&[
            (DEMO, "true"),
            (UMAMI_SCRIPT, "http://192.0.2.10:3000/stats/script.js?v=2"),
            ID,
        ]);
        assert_eq!(
            config.analytics.unwrap().unwrap().origin,
            "http://192.0.2.10:3000"
        );
    }

    #[test]
    fn a_setting_kamosu_cannot_use_counts_nothing_and_says_why() {
        for settings in [
            vec![(DEMO, "true"), SCRIPT],
            vec![(DEMO, "true"), ID],
            vec![
                (DEMO, "true"),
                (UMAMI_SCRIPT, "umami.example.com/script.js"),
                ID,
            ],
            vec![(DEMO, "true"), (UMAMI_SCRIPT, "javascript:alert(1)"), ID],
            vec![
                (DEMO, "true"),
                (UMAMI_SCRIPT, "https://a:b@umami.example.com/s.js"),
                ID,
            ],
            vec![(DEMO, "true"), (UMAMI_SCRIPT, "https://*/s.js"), ID],
            vec![
                (DEMO, "true"),
                (UMAMI_SCRIPT, "https://*.example.com/s.js"),
                ID,
            ],
            vec![
                (DEMO, "true"),
                (UMAMI_SCRIPT, "https://a.example;sandbox/s.js"),
                ID,
            ],
            vec![(DEMO, "true"), (UMAMI_SCRIPT, "http://[::1]:3000/s.js"), ID],
            vec![
                (DEMO, "true"),
                SCRIPT,
                (UMAMI_WEBSITE_ID, "x\" onload=\"alert(1)"),
            ],
            vec![
                (DEMO, "true"),
                SCRIPT,
                (UMAMI_WEBSITE_ID, "a; script-src *"),
            ],
        ] {
            let analytics = config(&settings).analytics;
            assert!(analytics.is_err(), "{settings:?} gave {analytics:?}");
        }
    }
}
