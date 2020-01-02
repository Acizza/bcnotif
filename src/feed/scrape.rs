use crate::err::ScrapeError;
use crate::feed::{Feed, Location};
use smallvec::SmallVec;

type Result<T> = std::result::Result<T, ScrapeError>;

pub fn scrape_top<S>(body: S) -> Result<Vec<Feed>>
where
    S: AsRef<str>,
{
    let table = {
        let mut body = body.as_ref();
        advance_str(&mut body, "<table class=\"btable\"")?;
        parse_tag_body_upto(body, "</table>")?
    };

    let mut feeds = Vec::with_capacity(50);

    for row in table.split("<tr>").skip(2) {
        let columns = parse_tr_entries(row, 3)?;

        let listeners =
            columns[0]
                .trim_end()
                .parse::<u32>()
                .map_err(|e| ScrapeError::FailedIntParse {
                    source: e,
                    element: "listeners",
                })?;

        let (location, county) = {
            let links = columns[1].splitn(3, "<a").collect::<SmallVec<[&str; 3]>>();

            if links.len() <= 1 {
                return Err(ScrapeError::NoLocationInfo);
            }

            let state_info = Link::parse(&links[1])?;
            let location = Location::with_state(state_info.href_id, state_info.value);

            let county = if links.len() > 2 {
                parse_tag_body(&links[2])?.to_string()
            } else {
                "Numerous".to_string()
            };

            (location, county)
        };

        let id_name_link = Link::parse(&columns[2])?;

        let alert = columns[2].find("<div").and_then(|idx| {
            let body = parse_tag_body_upto(&columns[2][idx..], "</div").ok()?;
            Some(body.to_string())
        });

        let feed = Feed {
            id: id_name_link.href_id,
            name: id_name_link.value.into(),
            listeners,
            location,
            county,
            alert,
        };

        feeds.push(feed);
    }

    if feeds.is_empty() {
        return Err(ScrapeError::NoFeeds);
    }

    Ok(feeds)
}

pub fn scrape_state<S>(state_id: u32, body: S) -> Result<Vec<Feed>>
where
    S: AsRef<str>,
{
    let table = {
        let mut body = body.as_ref();

        // State feeds have two tables with the same class
        advance_str(&mut body, "<table class=\"btable\"")?;
        advance_str(&mut body, "<table class=\"btable\"")?;

        parse_tag_body_upto(body, "</table>")?
    };

    let mut feeds = Vec::with_capacity(200);

    for row in table.split("<tr>").skip(2) {
        let columns = parse_tr_entries(row, 4)?;

        let county = parse_tag_body(&columns[0])?;
        let id_name_link = Link::parse(&columns[1])?;

        let alert = columns[1].find("<font").and_then(|idx| {
            let value = parse_tag_body_upto(&columns[1][idx..], "</font").ok()?;
            Some(value.to_string())
        });

        let listeners = get_str_upto_ch(&columns[3], '<')?
            .parse::<u32>()
            .map_err(|e| ScrapeError::FailedIntParse {
                source: e,
                element: "listeners",
            })?;

        let feed = Feed {
            id: id_name_link.href_id,
            name: id_name_link.value.into(),
            listeners,
            location: Location::new(state_id),
            county: county.into(),
            alert,
        };

        feeds.push(feed);
    }

    if feeds.is_empty() {
        return Err(ScrapeError::NoFeeds);
    }

    Ok(feeds)
}

fn parse_tr_entries(row: &str, num: usize) -> Result<SmallVec<[&str; 4]>> {
    let mut result = SmallVec::with_capacity(num);

    for split in row.splitn(num + 1, "<td").skip(1) {
        let value = parse_tag_body_upto(split, "</td")?;
        result.push(value);
    }

    if result.len() < num {
        return Err(ScrapeError::InvalidNumberOfColumns);
    }

    Ok(result)
}

fn advance_str(s: &mut &str, search_str: &str) -> Result<()> {
    let idx = s
        .find(search_str)
        .ok_or_else(|| ScrapeError::SearchStringNotFound {
            string: search_str.into(),
        })?;

    *s = &s[idx + search_str.len()..];

    Ok(())
}

fn advance_str_ch(s: &mut &str, search_ch: char) -> Result<()> {
    let idx = s
        .find(search_ch)
        .ok_or_else(|| ScrapeError::SearchStringNotFound {
            string: search_ch.to_string(),
        })?;

    *s = &s[idx + 1..];

    Ok(())
}

fn take_str_upto(s: &mut &str, end: &str) -> Result<()> {
    let idx = s
        .find(end)
        .ok_or_else(|| ScrapeError::SearchStringNotFound { string: end.into() })?;

    *s = &s[..idx];

    Ok(())
}

fn get_str_upto_ch(s: &str, end: char) -> Result<&str> {
    let idx = s
        .find(end)
        .ok_or_else(|| ScrapeError::SearchStringNotFound {
            string: end.to_string(),
        })?;

    let value = &s[..idx];

    Ok(value)
}

struct Link<'a> {
    href_id: u32,
    value: &'a str,
}

impl<'a> Link<'a> {
    fn new(href_id: u32, value: &'a str) -> Link<'a> {
        Link { href_id, value }
    }

    fn parse(mut body: &'a str) -> Result<Link<'a>> {
        advance_str(&mut body, "href=\"")?;

        let href_id = {
            let end = body
                .find('\"')
                .ok_or_else(|| ScrapeError::SearchStringNotFound {
                    string: "\"".into(),
                })?;

            let value = &body[..end];

            let id_idx = value
                .rfind('/')
                .ok_or_else(|| ScrapeError::SearchStringNotFound { string: "/".into() })?;

            let id =
                value[id_idx + 1..]
                    .parse::<u32>()
                    .map_err(|e| ScrapeError::FailedIntParse {
                        source: e,
                        element: "link id",
                    })?;

            // This assumes that the href attribute is the last one in the element
            body = &body[end + "\">".len()..];
            id
        };

        take_str_upto(&mut body, "</a")?;

        Ok(Link::new(href_id, body))
    }
}

fn parse_tag_body(body: &str) -> Result<&str> {
    parse_tag_body_upto(body, "</")
}

fn parse_tag_body_upto<'a>(mut body: &'a str, tag: &str) -> Result<&'a str> {
    advance_str_ch(&mut body, '>')?;
    take_str_upto(&mut body, tag)?;

    Ok(body)
}
