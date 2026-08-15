pub struct Attachment {
    pub id: u32,
    pub name: String,
    pub url: String,
    pub size: Option<String>,
    pub views: Option<u32>,
    pub media_id: Option<u32>,
    pub media_slug: Option<String>,
}
