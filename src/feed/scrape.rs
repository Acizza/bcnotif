extern crate select;

use ::feed::Feed;
use self::select::document::Document;
use self::select::node::Node;
use self::select::predicate::{Predicate, Class, Name};

error_chain! {
    errors {
        NoElement(element: String) {
            display("unable to find element that contains {} information", element)
        }

        FailedConvert(element: String) {
            display("unable to parse {} information", element)
        }

        NoneFound {
            display("no feeds found")
        }
    }
}

pub fn scrape_top(body: &str) -> Result<Vec<Feed>> {
    let doc = Document::from(body);

    let feed_data = doc
        .find(Class("btable").descendant(Name("tr")))
        .skip(1);

    let mut feeds = Vec::new();

    for row in feed_data {
        let (id, name) = parse_id_and_name(&row, "w100")?;

        let (state_id, county) = {
            // The top 50 feed list allows multiple states and/or counties to appear,
            // so we can't assume their location

            let location_info = row
                .find(Name("td"))
                .nth(1)
                .ok_or(ErrorKind::NoElement("location".into()))?;

            let mut hyperlinks = location_info
                .find(Name("a"))
                .filter_map(|link| {
                    link.attr("href").map(|url| (url, link.text()))
                });

            let state_id = hyperlinks
                .next()
                .and_then(|(link, _)| parse_link_id(&link))
                .ok_or(ErrorKind::NoElement("state id".into()))?
                .parse()
                .chain_err(|| ErrorKind::FailedConvert("state id".into()))?;

            let county = match hyperlinks.next() {
                Some((link, ref text)) if link.starts_with("/listen/ctid") => {
                    text.clone()
                },
                _ => "Numerous".to_string(),
            };

            (state_id, county)
        };

        feeds.push(Feed {
            id:        id,
            state_id:  state_id,
            county:    county,
            name:      name,
            listeners: parse_listeners(&row)?,
            alert:     row.find(Class("messageBox")).next().map(|alert| alert.text()),
        });
    }

    if feeds.len() == 0 {
        bail!(ErrorKind::NoneFound);
    }

    Ok(feeds)
}

pub fn scrape_state(state_id: u32, body: &str) -> Result<Vec<Feed>> {
    let doc = Document::from(body);

    // TODO: add support for areawide feeds
    let table = {
        // State feed pages may contain a section for areawide feeds that appears
        // before the main feed data. Since the parsing logic for that hasn't been
        // implemented yet, we simply skip over that table
        let tables = doc.find(Class("btable"))
            .take(2)
            .collect::<Vec<_>>();

        if tables.len() == 0 {
            bail!(ErrorKind::NoElement("feed data".into()));
        } else if tables.len() >= 2 {
            tables[1]
        } else {
            tables[0]
        }
    };

    let feed_data = table
        .find(Class("btable").descendant(Name("tr")));

    let mut feeds = Vec::new();

    for feed in feed_data.skip(1) {
        let (id, name) = parse_id_and_name(&feed, "w1p")?;

        let county = feed
            .find(Name("a"))
            .next()
            .map(|node| node.text())
            .unwrap_or("Numerous".to_string());

        let alert = feed
            .find(Name("font").and(Class("fontRed")))
            .next()
            .map(|alert| alert.text());

        feeds.push(Feed {
            id:        id,
            state_id:  state_id,
            county:    county,
            name:      name,
            listeners: parse_listeners(&feed)?,
            alert:     alert,
        });
    }

    if feeds.len() == 0 {
        bail!(ErrorKind::NoneFound);
    }

    Ok(feeds)
}

fn parse_id_and_name(node: &Node, class_name: &str) -> Result<(u32, String)> {
    let base = node
        .find(Class(class_name).descendant(Name("a")))
        .next()
        .ok_or(ErrorKind::NoElement("id and name".into()))?;

    let id = base
        .attr("href")
        .and_then(parse_link_id)
        .ok_or(ErrorKind::NoElement("feed id".into()))?
        .parse()
        .chain_err(|| ErrorKind::FailedConvert("state id".into()))?;

    Ok((id, base.text()))
}

fn parse_listeners(node: &Node) -> Result<u32> {
    let text =
        node.find(Class("c").and(Class("m")))
            .next()
            .map(|node| node.text())
            .ok_or(ErrorKind::NoElement("feed listeners".into()))?;

    text.trim_right()
        .parse()
        .chain_err(|| ErrorKind::FailedConvert("feed listeners".into()))
}

fn parse_link_id(url: &str) -> Option<String> {
    let pos = url.rfind('/')?;
    
    if pos + 1 >= url.len() {
        None
    } else {
        Some(url[pos + 1..].to_string())
    }
}