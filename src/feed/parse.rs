extern crate select;

use std::error::{self, Error};
use std::fmt;
use std::num;
use feed::Feed;
use util::error::DetailedError;
use self::select::document::Document;
use self::select::node::Node;
use self::select::predicate::{Predicate, Name, Class};

pub fn top_feeds(html: &str) -> Result<Vec<Feed>, DetailedError> {
    let doc = Document::from(strip_front(html, "<table class=\"btable", 1));
    let feed_data = doc.find(Class("btable").descendant(Name("tr")));

    let mut feeds = Vec::new();

    for feed in feed_data.skip(1) {
        let (id, name) = try_detailed!(parse_id_and_name, &feed, "w100");

        let (state_id, county) = {
            // The top 50 feed list allows multiple states and/or counties to appear,
            // so we can't assume their location
            let hyperlinks = try_detailed!(feed
                .find(Name("td"))
                .nth(1)
                .ok_or("unable to get feed location links"));

            let mut hyperlinks = hyperlinks
                .find(Name("a"))
                .filter_map(|link| {
                    link.attr("href").map(|url| (url, link.text()))
                });

            let state_id = try_detailed!(hyperlinks
                .next()
                .and_then(|(link, _)| parse_link_id(&link))
                .ok_or("unable to get feed state id".into())
                .and_then(|id| id.parse().map_err(ParseError::from)));

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
            listeners: try_detailed!(parse_listeners, &feed),
            alert:     feed.find(Class("messageBox")).next().map(|alert| alert.text()),
        });
    }

    Ok(feeds)
}

pub fn state_feeds(html: &str, state_id: u32) -> Result<Vec<Feed>, DetailedError> {
    let doc = Document::from(strip_front(html, "<table class=\"btable", 2));
    let feed_data = doc.find(Class("btable").descendant(Name("tr")));

    let mut feeds = Vec::new();

    for feed in feed_data.skip(1) {
        let (id, name) = try_detailed!(parse_id_and_name, &feed, "w1p");

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
            listeners: try_detailed!(parse_listeners, &feed),
            alert:     alert,
        });
    }

    Ok(feeds)
}

fn parse_link_id(url: &str) -> Option<String> {
    let pos = try_opt!(url.rfind('/'));
    
    if pos + 1 >= url.len() {
        None
    } else {
        Some(url[pos + 1..].to_string())
    }
}

fn strip_front<'a>(string: &'a str, delim: &str, num_times: u32) -> &'a str {
    let mut slice = string;

    for _ in 0..num_times {
        match slice[delim.len()..].find(delim) {
            Some(p) => slice = &slice[p + delim.len()..],
            None    => break,
        }
    }

    slice
}

fn parse_id_and_name(node: &Node, class_name: &str) -> Result<(u32, String), ParseError> {
    let base = node
        .find(Class(class_name).descendant(Name("a")))
        .next()
        .ok_or("unable to get base for feed id & name")?;

    let id = base
        .attr("href")
        .and_then(parse_link_id)
        .ok_or("unable to get feed id")?
        .parse()?;

    Ok((id, base.text()))
}

fn parse_listeners(node: &Node) -> Result<u32, ParseError> {
    let text =
        node.find(Class("c").and(Class("m")))
            .next()
            .map(|node| node.text())
            .ok_or("unable to get feed listeners")?;

    Ok(text.trim_right().parse()?)
}

#[derive(Debug)]
pub struct ParseError {
    pub reason: String,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "feed parsing error: {}", self.reason)
    }
}

impl error::Error for ParseError {
    fn description(&self) -> &str {
        &self.reason
    }
}

impl From<num::ParseIntError> for ParseError {
    fn from(err: num::ParseIntError) -> ParseError {
        ParseError { reason: err.description().to_string() }
    }
}

impl<'a> From<&'a str> for ParseError {
    fn from(err: &'a str) -> ParseError {
        ParseError { reason: err.to_string() }
    }
}