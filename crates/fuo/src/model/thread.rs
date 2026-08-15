use crate::model::attachment::Attachment;

pub struct Thread {
    pub id: u32,
    pub slug: String,
    pub title: String,
    pub prefix: Option<String>,
    pub prefix_class: Option<String>,
    pub author: String,
    pub replies: u32,
    pub url: String,
}

pub struct ForumPage {
    pub threads: Vec<Thread>,
    pub page: u32,
    pub total_pages: u32,
}

pub struct ThreadPage {
    pub title: String,
    pub attachments: Vec<Attachment>,
}
