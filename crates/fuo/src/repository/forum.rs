use crate::model::thread::Thread;
use scraper::{Html, Selector};
use std::sync::LazyLock;

static ITEM: LazyLock<Selector> = LazyLock::new(|| {
    Selector::parse(".js-threadList > .structItem--thread").expect("valid selector")
});
static TITLE: LazyLock<Selector> = LazyLock::new(|| {
    Selector::parse(".structItem-title > a:not(.labelLink)").expect("valid selector")
});
static PREFIX: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse(".structItem-title .label").expect("valid selector"));
static AUTHOR: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse(".structItem-parts .username").expect("valid selector"));
static REPLIES: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse(".structItem-cell--meta dd").expect("valid selector"));
static PAGE_NAV: LazyLock<Selector> = LazyLock::new(|| {
    Selector::parse(".pageNav-main li.pageNav-page:not(.pageNav-page--skip)")
        .expect("valid selector")
});

pub fn parse_threads(html: &str) -> Vec<Thread> {
    let doc = Html::parse_document(html);

    doc.select(&ITEM)
        .map(|row| {
            let id = thread_id(row.value().attr("class").unwrap_or_default());
            let title_link = row.select(&TITLE).next();
            let slug = title_link
                .and_then(|a| a.value().attr("href"))
                .and_then(thread_slug)
                .unwrap_or_default();

            let label = row.select(&PREFIX).next();

            Thread {
                id,
                slug: slug.clone(),
                title: title_link
                    .map(|a| a.text().collect::<String>().trim().to_owned())
                    .unwrap_or_default(),
                prefix: label
                    .as_ref()
                    .map(|span| span.text().collect::<String>().trim().to_owned()),
                prefix_class: label.as_ref().and_then(|span| {
                    span.value().attr("class").and_then(|classes| {
                        classes
                            .split_whitespace()
                            .find(|class| class.starts_with("label--"))
                            .map(std::borrow::ToOwned::to_owned)
                    })
                }),
                author: row
                    .select(&AUTHOR)
                    .next()
                    .map(|a| a.text().collect::<String>().trim().to_owned())
                    .unwrap_or_default(),
                replies: row
                    .select(&REPLIES)
                    .next()
                    .map(|dd| parse_replies(&dd.text().collect::<String>()))
                    .unwrap_or_default(),
                url: format!("/threads/{slug}.{id}/"),
            }
        })
        .collect()
}

pub fn total_pages(html: &str) -> u32 {
    let doc = Html::parse_document(html);

    doc.select(&PAGE_NAV)
        .filter_map(|li| li.text().collect::<String>().trim().parse::<u32>().ok())
        .max()
        .unwrap_or(1)
}

fn thread_id(classes: &str) -> u32 {
    classes
        .split_whitespace()
        .find_map(|class| {
            class
                .strip_prefix("js-threadListItem-")
                .and_then(|id| id.parse().ok())
        })
        .unwrap_or_default()
}

fn thread_slug(href: &str) -> Option<String> {
    href.trim_start_matches("/threads/")
        .split('/')
        .next()
        .and_then(|segment| segment.rsplit_once('.').map(|(slug, _)| slug.to_owned()))
}

fn parse_replies(text: &str) -> u32 {
    let text = text.trim();
    text.strip_suffix('K').map_or_else(
        || text.parse().unwrap_or_default(),
        |k| (k.trim().parse::<f32>().unwrap_or_default() * 1000.0) as u32,
    )
}
