//! A link to a video, told apart from the codes around it (#464). Instagram,
//! YouTube and TikTok add a code to a link each time it is shared, naming who
//! shared it, and Instagram does not even keep the code's name the same. So
//! two shares of one reel are two strings, and only the part that names the
//! post says they are one.
//!
//! [`link_identity`] is what two links are compared by, and [`without_tracking`]
//! is what a recipe's Source keeps. Every other link is compared and kept as
//! it is. The real shapes are in `tests/fixtures/shared-links.json`.

use rusqlite::Connection;
use rusqlite::functions::FunctionFlags;
use url::Url;

/// One link to a post on a site Kamosu knows the shape of.
struct Video {
    url: Url,
    /// The one address every link to this post comes down to.
    identity: String,
    /// What in its query is not a code, and stays in a recipe's Source.
    keeps: &'static [&'static str],
}

/// What `link` is matched by: the one address of its post, or `link` itself.
pub fn link_identity(link: &str) -> String {
    video(link).map_or_else(|| link.to_string(), |video| video.identity)
}

/// `link` as a recipe's Source keeps it: with no code on a video site, and
/// as it is anywhere else.
pub fn without_tracking(link: &str) -> String {
    let Some(Video { mut url, keeps, .. }) = video(link) else {
        return link.to_string();
    };
    let kept: Vec<(String, String)> = url
        .query_pairs()
        .filter(|(name, _)| keeps.contains(&name.as_ref()))
        .map(|(name, value)| (name.into_owned(), value.into_owned()))
        .collect();
    url.set_fragment(None);
    url.set_query(None);
    if !kept.is_empty() {
        url.query_pairs_mut().extend_pairs(kept);
    }
    url.into()
}

fn video(link: &str) -> Option<Video> {
    let url = Url::parse(link).ok()?;
    if !matches!(url.scheme(), "http" | "https") {
        return None;
    }
    let host = url.host_str()?;
    let host = host.strip_prefix("www.").unwrap_or(host);
    let path: Vec<&str> = url.path_segments()?.filter(|s| !s.is_empty()).collect();
    let (identity, keeps): (String, &[&str]) = match host {
        "instagram.com" | "m.instagram.com" => (instagram_post(&path)?, &[]),
        "youtube.com" | "m.youtube.com" => (youtube_video(&url, &path)?, &["v", "t"]),
        "youtu.be" => (youtube_address(path.first()?)?, &["t"]),
        "tiktok.com" => (tiktok_video(&path)?, &[]),
        _ => return None,
    };
    Some(Video {
        url,
        identity,
        keeps,
    })
}

/// `/reel/<id>`, `/p/<id>` and the like, which are one post under any of
/// them, with or without its author's name in front. Not a sound's page, and
/// not a `/share/` address, whose token only leads to a post.
fn instagram_post(path: &[&str]) -> Option<String> {
    let id = match path {
        ["reels", "audio", ..] | ["share", ..] => return None,
        ["reel" | "reels" | "p" | "tv", id, ..] => id,
        [_, "reel" | "reels" | "p" | "tv", id, ..] => id,
        _ => return None,
    };
    is_id(id).then(|| format!("https://www.instagram.com/p/{id}/"))
}

fn youtube_video(url: &Url, path: &[&str]) -> Option<String> {
    match path {
        ["watch"] => {
            let (_, id) = url.query_pairs().find(|(name, _)| name == "v")?;
            youtube_address(&id)
        }
        ["shorts" | "live", id, ..] => youtube_address(id),
        _ => None,
    }
}

fn youtube_address(id: &str) -> Option<String> {
    is_id(id).then(|| format!("https://www.youtube.com/watch?v={id}"))
}

/// `/@<user>/video/<id>`. The id alone names the video: a cook who changes
/// their name leaves the same video under another address.
fn tiktok_video(path: &[&str]) -> Option<String> {
    match path {
        [user, "video", id, ..] if user.starts_with('@') && is_id(id) => {
            Some(format!("https://www.tiktok.com/video/{id}"))
        }
        _ => None,
    }
}

fn is_id(text: &str) -> bool {
    !text.is_empty()
        && text
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// Teach one SQLite connection `link_identity(link)`, so the links a query
/// reads are compared as Kamosu compares them. Registered before the
/// migrations run because one of them needs it. Anything but text answers
/// NULL, which matches nothing.
pub fn register(conn: &Connection) -> rusqlite::Result<()> {
    conn.create_scalar_function(
        "link_identity",
        1,
        FunctionFlags::SQLITE_UTF8 | FunctionFlags::SQLITE_DETERMINISTIC,
        |ctx| Ok(ctx.get_raw(0).as_str().ok().map(link_identity)),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn fixture() -> Value {
        serde_json::from_str(include_str!("../tests/fixtures/shared-links.json")).unwrap()
    }

    #[test]
    fn each_link_a_phone_really_shared_comes_down_to_its_post() {
        let fixture = fixture();
        for row in fixture["links"].as_array().unwrap() {
            let link = row["link"].as_str().unwrap();
            assert_eq!(
                link_identity(link),
                row["identity"].as_str().unwrap(),
                "{link}"
            );
            assert_eq!(
                without_tracking(link),
                row["source"].as_str().unwrap(),
                "{link}"
            );
        }
    }

    #[test]
    fn a_link_to_anything_else_is_left_as_it_is() {
        let fixture = fixture();
        for link in fixture["left_as_they_are"].as_array().unwrap() {
            let link = link.as_str().unwrap();
            assert_eq!(link_identity(link), link);
            assert_eq!(without_tracking(link), link);
        }
    }

    /// A code under a name no one has seen yet: nothing lists the codes.
    #[test]
    fn a_new_code_name_changes_nothing() {
        let reel = "https://www.instagram.com/reel/POSTID/?zzzz=CODE";
        assert_eq!(link_identity(reel), "https://www.instagram.com/p/POSTID/");
        assert_eq!(
            without_tracking(reel),
            "https://www.instagram.com/reel/POSTID/"
        );
    }

    /// A reel, a post and an address with its author's name are one post, on
    /// any of the site's hosts, whatever is after the `#`.
    #[test]
    fn one_post_is_one_post_under_every_address() {
        for link in [
            "https://instagram.com/reel/POSTID",
            "http://m.instagram.com/reels/POSTID/",
            "https://www.instagram.com/somecook/reel/POSTID/#comments",
            "https://www.instagram.com/somecook/p/POSTID/",
            "https://www.instagram.com/tv/POSTID/",
        ] {
            assert_eq!(
                link_identity(link),
                "https://www.instagram.com/p/POSTID/",
                "{link}"
            );
        }
        for link in [
            "https://m.youtube.com/watch?t=95&v=VIDEOID",
            "https://www.youtube.com/watch?v=VIDEOID&list=CODE",
            "https://www.youtube.com/live/VIDEOID?si=CODE",
        ] {
            assert_eq!(
                link_identity(link),
                "https://www.youtube.com/watch?v=VIDEOID",
                "{link}"
            );
        }
    }

    /// The start time is the cook's, and it is not a code. TikTok's `t` is.
    #[test]
    fn a_start_time_stays_in_the_source_of_a_youtube_link_alone() {
        assert_eq!(
            without_tracking("https://youtu.be/VIDEOID?si=CODE&t=95"),
            "https://youtu.be/VIDEOID?t=95"
        );
        assert_eq!(
            without_tracking("https://www.tiktok.com/@somecook/video/7000?t=CODE"),
            "https://www.tiktok.com/@somecook/video/7000"
        );
    }

    #[test]
    fn sqlite_compares_links_the_same_way() {
        let conn = Connection::open_in_memory().unwrap();
        register(&conn).unwrap();
        let answer = |sql: &str| {
            conn.query_row(sql, [], |row| row.get::<_, Option<String>>(0))
                .unwrap()
        };
        assert_eq!(
            answer("SELECT link_identity('https://youtu.be/VIDEOID?si=CODE')").as_deref(),
            Some("https://www.youtube.com/watch?v=VIDEOID")
        );
        assert_eq!(answer("SELECT link_identity(NULL)"), None);
        assert_eq!(answer("SELECT link_identity(4)"), None);
    }
}
