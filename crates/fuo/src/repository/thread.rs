use std::sync::LazyLock;

use scraper::{Html, Selector};

use crate::model::attachment::Attachment;
use crate::model::comment::Comment;

static TITLE: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse(".p-title-value").expect("valid selector"));
static LABEL: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse(".label").expect("valid selector"));
static ATTACHMENT_ITEM: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("ul.attachmentList li.file").expect("valid selector"));
static ATTACHMENT_LINK: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("a.file-preview").expect("valid selector"));
static ATTACHMENT_NAME: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse(".file-name").expect("valid selector"));
static ATTACHMENT_META: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse(".file-meta").expect("valid selector"));

static COMMENT_ITEM: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("div.js-comment").expect("valid selector"));
static COMMENT_AUTHOR: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse(".comment-user").expect("valid selector"));
static COMMENT_BODY: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse(".comment-body").expect("valid selector"));
static COMMENT_DATE: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse("time.u-dt").expect("valid selector"));
static COMMENT_VOTE: LazyLock<Selector> =
    LazyLock::new(|| Selector::parse(".user-fuo-vote").expect("valid selector"));

pub fn parse_thread_title(html: &str) -> String {
    let doc = Html::parse_document(html);

    let Some(el) = doc.select(&TITLE).next() else {
        return String::new();
    };
    let title = el.text().collect::<String>().trim().to_owned();

    let label = el
        .select(&LABEL)
        .next()
        .map(|span| span.text().collect::<String>().trim().to_owned())
        .unwrap_or_default();
    match title.strip_prefix(&label) {
        Some(rest) if !label.is_empty() => rest.trim().to_owned(),
        _ => title,
    }
}

pub fn parse_attachments(html: &str) -> Vec<Attachment> {
    let doc = Html::parse_document(html);

    doc.select(&ATTACHMENT_ITEM)
        .map(|li| {
            let link_el = li.select(&ATTACHMENT_LINK).next();
            let href = link_el
                .and_then(|a| a.value().attr("href"))
                .unwrap_or_default();

            let id = href
                .trim_start_matches("/attachments/")
                .split('/')
                .next()
                .and_then(|segment| segment.rsplit_once('.'))
                .and_then(|(_, id)| id.parse().ok())
                .unwrap_or_default();

            let (size, views) = parse_meta(
                &li.select(&ATTACHMENT_META)
                    .next()
                    .map(|el| el.text().collect::<String>())
                    .unwrap_or_default(),
            );

            let media = link_el
                .and_then(|a| a.value().attr("data-lb-sidebar-href"))
                .and_then(parse_media);

            Attachment {
                id,
                name: li
                    .select(&ATTACHMENT_NAME)
                    .next()
                    .map(|el| el.text().collect::<String>().trim().to_owned())
                    .unwrap_or_default(),
                url: href.to_owned(),
                size,
                views,
                media_id: media.as_ref().map(|(_, id)| *id),
                media_slug: media.map(|(slug, _)| slug),
            }
        })
        .collect()
}

pub fn parse_comments(html: &str) -> Vec<Comment> {
    let doc = Html::parse_document(html);

    doc.select(&COMMENT_ITEM)
        .map(|comment| {
            let author_name = comment
                .select(&COMMENT_AUTHOR)
                .next()
                .map(|el| el.text().collect::<String>().trim().to_owned())
                .or_else(|| comment.value().attr("data-author").map(str::to_owned))
                .unwrap_or_default();

            Comment {
                author: author_name,
                body: comment
                    .select(&COMMENT_BODY)
                    .next()
                    .map(|el| el.text().collect::<String>().trim().to_owned())
                    .unwrap_or_default(),
                date: comment
                    .select(&COMMENT_DATE)
                    .next()
                    .and_then(|el| el.value().attr("datetime"))
                    .unwrap_or_default()
                    .to_owned(),
                vote: comment
                    .select(&COMMENT_VOTE)
                    .next()
                    .map(|el| el.text().collect::<String>().trim().to_owned()),
            }
        })
        .collect()
}

fn parse_meta(meta: &str) -> (Option<String>, Option<u32>) {
    let mut size = None;
    let mut views = None;
    for part in meta.split('·') {
        let part = part.trim();
        if let Some(views_text) = part.strip_prefix("Xem:") {
            views = views_text.trim().parse().ok();
        } else if !part.is_empty() {
            size = Some(part.to_owned());
        }
    }
    (size, views)
}

fn parse_media(href: &str) -> Option<(String, u32)> {
    let segment = href.trim_start_matches("/media/").split('/').next()?;
    let (slug, id) = segment.rsplit_once('.')?;
    id.parse().ok().map(|id| (slug.to_owned(), id))
}
