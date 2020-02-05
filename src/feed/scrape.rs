use crate::err::ScrapeError;
use crate::feed::{Feed, Location};
use smallvec::SmallVec;

type Result<T> = std::result::Result<T, ScrapeError>;

macro_rules! try_cont {
    ($x:expr) => {
        match $x {
            Some(value) => value,
            None => continue,
        }
    };
}

macro_rules! try_cont_r {
    ($x:expr) => {
        match $x {
            Ok(value) => value,
            Err(_) => continue,
        }
    };
}

pub fn scrape_top<'a, S>(body: S, min_listeners: u32) -> Result<Vec<Feed<'a>>>
where
    S: AsRef<str>,
{
    let body = body.as_ref();
    let feed_table = tag_body_find(body, "<table class=\"btable\"", "</table>")
        .ok_or(ScrapeError::MissingFeedTable)?;

    let mut feeds = Vec::with_capacity(50);

    for row in feed_table.split("<tr>").skip(2) {
        let columns = try_cont!(tr_columns(row, 3));
        let listeners = try_cont_r!(columns[0].trim_end().parse());

        if listeners < min_listeners {
            continue;
        }

        let (location, county) = {
            let links = columns[1].splitn(3, "<a").collect::<SmallVec<[&str; 3]>>();

            if links.len() <= 1 {
                continue;
            }

            let state_info = try_cont!(Link::parse(&links[1]));
            let location = Location::with_state(state_info.href_id, state_info.value);

            let county = if links.len() > 2 {
                try_cont!(tag_body(&links[2], "</")).to_string().into()
            } else {
                "Numerous".into()
            };

            (location, county)
        };

        let id_name_link = try_cont!(Link::parse(&columns[2]));

        let alert = columns[2].find("<div").and_then(|pos| {
            let body = tag_body(&columns[2][pos..], "</div")?;
            Some(body.into())
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

pub fn scrape_state<'a, S>(body: S, min_listeners: u32, state_id: u32) -> Result<Vec<Feed<'a>>>
where
    S: AsRef<str>,
{
    let body = body.as_ref();

    let feed_table = {
        let next_table = |slice| {
            slice_from(slice, "<table class=\"btable\"").ok_or(ScrapeError::MissingFeedTable)
        };

        // Feeds in the United States are in the second table, and in the third table otherwise.
        let first = next_table(body)?;
        let second = next_table(first)?;

        let feed_table = match next_table(second) {
            Ok(table) => table,
            Err(_) => second,
        };

        tag_body(feed_table, "</table>").ok_or(ScrapeError::MissingFeedTable)?
    };

    let mut feeds = Vec::with_capacity(200);

    for row in feed_table.split("<tr>").skip(2) {
        let columns = try_cont!(tr_columns(row, 4));
        let listeners = try_cont!(slice_to_ch(&columns[3], '<').and_then(|v| v.parse().ok()));

        if listeners < min_listeners {
            continue;
        }

        let county = try_cont!(tag_body(&columns[0], "</")).to_string().into();
        let id_name_link = try_cont!(Link::parse(&columns[1]));

        let alert = columns[1].find("<font").and_then(|pos| {
            let body = tag_body(&columns[1][pos..], "</font")?;
            Some(body.into())
        });

        let feed = Feed {
            id: id_name_link.href_id,
            name: id_name_link.value.into(),
            listeners,
            location: Location::new(state_id),
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

#[inline(always)]
fn tag_body_find<'a>(string: &'a str, start: &str, end: &str) -> Option<&'a str> {
    let start = slice_from(string, start)?;
    let body = slice_from_ch(start, '>')?;
    let slice = slice_to(body, end)?;
    Some(slice)
}

#[inline(always)]
fn tag_body<'a>(string: &'a str, end: &str) -> Option<&'a str> {
    let body = slice_from_ch(string, '>')?;
    let slice = slice_to(body, end)?;
    Some(slice)
}

fn tr_columns(row: &str, num: usize) -> Option<SmallVec<[&str; 4]>> {
    let mut columns = SmallVec::new();

    for column in row.splitn(num + 1, "<td").skip(1) {
        let column = tag_body(column, "</td>")?;
        columns.push(column);
    }

    if columns.len() < num {
        return None;
    }

    Some(columns)
}

#[inline(always)]
fn slice_from<'a>(string: &'a str, start: &str) -> Option<&'a str> {
    string.find(start).map(|pos| &string[pos + start.len()..])
}

#[inline(always)]
fn slice_from_ch(string: &str, start: char) -> Option<&str> {
    string.find(start).map(|pos| &string[pos + 1..])
}

#[inline(always)]
fn slice_to<'a>(string: &'a str, end: &str) -> Option<&'a str> {
    string.find(end).map(|pos| &string[..pos])
}

#[inline(always)]
fn slice_to_ch(string: &str, end: char) -> Option<&str> {
    string.find(end).map(|pos| &string[..pos])
}

struct Link<'a> {
    href_id: u32,
    value: &'a str,
}

impl<'a> Link<'a> {
    fn new(href_id: u32, value: &'a str) -> Self {
        Link { href_id, value }
    }

    fn parse(body: &'a str) -> Option<Self> {
        let href_start = slice_from(body, "href=\"")?;

        let id_end_pos = href_start.find('\"')?;
        let id_value = &href_start[..id_end_pos];
        let id_start_pos = id_value.rfind('/')?;
        let href_id = id_value[id_start_pos + 1..].parse().ok()?;

        // This assumes that the href attribute is the last one in the element
        let value = slice_to(&href_start[id_end_pos + "\">".len()..], "</a")?;

        Some(Self::new(href_id, value))
    }
}
