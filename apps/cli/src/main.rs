use std::fmt::Write as _;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::Result;
use clap::{Parser, Subcommand};
use fuo::prelude::{Attachment, Comment, FuoClient, Thread};

#[derive(Parser)]
#[command(version, about = "FuOverflow scraper")]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Login {
        login: Option<String>,
        password: Option<String>,
    },
    Search {
        subject: String,
        filter: Option<String>,
        #[arg(long)]
        login: Option<String>,
        #[arg(long)]
        password: Option<String>,
    },
    Logout,
    Install {
        target: String,
        #[arg(long)]
        no_comments: bool,
        #[arg(long, default_value = "install")]
        dir: String,
        #[arg(long, default_value_t = 500)]
        delay_ms: u64,
    },
    Thread {
        id: u32,
        #[arg(long)]
        download: bool,
        #[arg(long)]
        comments: bool,
        #[arg(long, default_value = "downloads")]
        dir: String,
        #[arg(long, default_value_t = 500)]
        delay_ms: u64,
    },
}

const RESET: &str = "\x1b[0m";

fn session_path() -> Result<PathBuf> {
    let exe = std::env::current_exe()?;
    let dir = exe.parent().unwrap_or_else(|| Path::new("."));
    Ok(dir.join("session.txt"))
}

fn expand_tilde(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        let home = std::env::var("USERPROFILE")
            .or_else(|_| std::env::var("HOME"))
            .unwrap_or_default();
        if !home.is_empty() {
            return Path::new(&home).join(rest);
        }
    }
    PathBuf::from(path)
}

fn is_forbidden(err: &anyhow::Error) -> bool {
    err.downcast_ref::<fuo::prelude::Error>()
        .is_some_and(|e| matches!(e, fuo::prelude::Error::Network(n) if n.status().map(|s| s.as_u16()) == Some(403)))
}

fn save_session(client: &FuoClient) -> Result<()> {
    std::fs::write(session_path()?, client.session_cookies())?;
    Ok(())
}

fn load_session() -> Result<Option<String>> {
    let path = session_path()?;
    if path.exists() {
        Ok(Some(std::fs::read_to_string(path)?))
    } else {
        Ok(None)
    }
}

fn prompt(text: &str) -> Result<String> {
    print!("{text}");
    std::io::stdout().flush()?;
    let mut line = String::new();
    std::io::stdin().read_line(&mut line)?;
    Ok(line.trim().to_owned())
}

fn short_prefix(prefix: &str) -> &str {
    prefix.strip_prefix("Đề Thi ").unwrap_or(prefix)
}

fn color_for(prefix_class: Option<&str>) -> &'static str {
    match prefix_class.unwrap_or_default() {
        "label--red" => "\x1b[31m",
        "label--orange" => "\x1b[33m",
        "label--green" => "\x1b[32m",
        "label--blue" => "\x1b[34m",
        "label--purple" => "\x1b[35m",
        "label--gray" => "\x1b[90m",
        _ => "",
    }
}

fn print_threads(threads: &[Thread]) {
    if threads.is_empty() {
        println!("No threads found.");
        return;
    }

    let id_w = threads
        .iter()
        .map(|t| t.id.to_string().len())
        .max()
        .unwrap_or(2)
        .max(2);
    let prefix_w = threads
        .iter()
        .map(|t| {
            short_prefix(t.prefix.as_deref().unwrap_or("-"))
                .chars()
                .count()
        })
        .max()
        .unwrap_or(2)
        .max(6);
    let title_w = threads
        .iter()
        .map(|t| t.title.chars().count())
        .max()
        .unwrap_or(5)
        .max(5);
    let author_w = threads
        .iter()
        .map(|t| t.author.chars().count())
        .max()
        .unwrap_or(6)
        .max(6);
    let replies_w = threads
        .iter()
        .map(|t| t.replies.to_string().len())
        .max()
        .unwrap_or(7)
        .max(7);

    println!(
        "{:<id_w$}  {:<prefix_w$}  {:<title_w$}  {:<author_w$}  {:<replies_w$}",
        "ID",
        "PREFIX",
        "TITLE",
        "AUTHOR",
        "REPLIES",
        id_w = id_w,
        prefix_w = prefix_w,
        title_w = title_w,
        author_w = author_w,
        replies_w = replies_w,
    );

    for thread in threads {
        let prefix = short_prefix(thread.prefix.as_deref().unwrap_or("-"));
        let padded = format!("{prefix:<prefix_w$}");
        let color = color_for(thread.prefix_class.as_deref());
        println!(
            "{:<id_w$}  {color}{padded}{RESET}  {:<title_w$}  {:<author_w$}  {:>replies_w$}",
            thread.id,
            thread.title,
            thread.author,
            thread.replies,
            id_w = id_w,
            title_w = title_w,
            author_w = author_w,
            replies_w = replies_w,
        );
    }

    println!("\n{} threads", threads.len());
}

fn print_attachments(attachments: &[Attachment]) {
    if attachments.is_empty() {
        println!("No attachments.");
        return;
    }

    let id_w = attachments
        .iter()
        .map(|a| a.id.to_string().len())
        .max()
        .unwrap_or(2)
        .max(2);
    let name_w = attachments
        .iter()
        .map(|a| a.name.chars().count())
        .max()
        .unwrap_or(4)
        .max(4);
    let size_w = attachments
        .iter()
        .map(|a| a.size.as_deref().unwrap_or("-").chars().count())
        .max()
        .unwrap_or(4)
        .max(4);
    let views_w = attachments
        .iter()
        .map(|a| a.views.unwrap_or_default().to_string().len())
        .max()
        .unwrap_or(5)
        .max(5);

    println!(
        "{:<id_w$}  {:<name_w$}  {:<size_w$}  {:<views_w$}",
        "ID",
        "NAME",
        "SIZE",
        "VIEWS",
        id_w = id_w,
        name_w = name_w,
        size_w = size_w,
        views_w = views_w,
    );
    for attachment in attachments {
        println!(
            "{:<id_w$}  {:<name_w$}  {:<size_w$}  {:<views_w$}",
            attachment.id,
            attachment.name,
            attachment.size.as_deref().unwrap_or("-"),
            attachment
                .views
                .map_or_else(|| "-".to_owned(), |v| v.to_string()),
            id_w = id_w,
            name_w = name_w,
            size_w = size_w,
            views_w = views_w,
        );
    }

    println!("\n{} attachments", attachments.len());
}

fn vote_suffix(vote: Option<&str>) -> String {
    match vote {
        Some(vote) if !vote.is_empty() => format!(" [vote {vote}]"),
        _ => String::new(),
    }
}

fn toml_key(stem: &str, id: u32) -> String {
    let safe = !stem.is_empty()
        && stem
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
    if safe {
        stem.to_owned()
    } else {
        id.to_string()
    }
}

fn escape_toml(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            _ => out.push(c),
        }
    }
    out
}

fn write_manifest_toml(title: &str, id: u32, attachments: &[Attachment]) -> String {
    let mut out = String::new();
    writeln!(out, "title = \"{}\"", escape_toml(title)).unwrap();
    writeln!(out, "thread_id = {id}").unwrap();

    for attachment in attachments {
        let stem = attachment
            .name
            .rsplit_once('.')
            .map_or(attachment.name.as_str(), |(stem, _)| stem);
        let key = toml_key(stem, attachment.id);

        writeln!(out).unwrap();
        writeln!(out, "[{key}]").unwrap();
        writeln!(out, "name = \"{}\"", escape_toml(&attachment.name)).unwrap();
        writeln!(out, "attachment_id = {}", attachment.id).unwrap();
        if let Some(size) = &attachment.size {
            writeln!(out, "size = \"{}\"", escape_toml(size)).unwrap();
        }
        if let Some(views) = attachment.views {
            writeln!(out, "views = {views}").unwrap();
        }
    }
    out
}

fn write_comments_toml(title: &str, id: u32, items: &[(&Attachment, Vec<Comment>)]) -> String {
    let mut out = String::new();
    writeln!(out, "title = \"{}\"", escape_toml(title)).unwrap();
    writeln!(out, "thread_id = {id}").unwrap();

    for (attachment, comments) in items {
        if comments.is_empty() {
            continue;
        }
        let stem = attachment
            .name
            .rsplit_once('.')
            .map_or(attachment.name.as_str(), |(stem, _)| stem);
        let key = toml_key(stem, attachment.id);

        writeln!(out).unwrap();
        writeln!(out, "[{key}]").unwrap();
        for comment in comments {
            writeln!(out, "[[{key}.comments]]").unwrap();
            writeln!(out, "author = \"{}\"", escape_toml(&comment.author)).unwrap();
            writeln!(out, "date = \"{}\"", escape_toml(&comment.date)).unwrap();
            if let Some(vote) = &comment.vote {
                writeln!(out, "vote = \"{}\"", escape_toml(vote)).unwrap();
            }
            writeln!(out, "body = \"{}\"", escape_toml(&comment.body)).unwrap();
        }
    }
    out
}

async fn install_thread(
    client: &FuoClient,
    id: u32,
    root: &Path,
    with_comments: bool,
) -> Result<Option<(usize, usize, usize)>> {
    let page = client.thread_page(id).await?;
    let thread_dir = root.join(id.to_string());

    if thread_dir.join("manifest.toml").exists() {
        return Ok(None);
    }
    std::fs::create_dir_all(&thread_dir)?;

    let mut items: Vec<(&Attachment, Vec<Comment>)> = Vec::new();
    let mut commented = 0;
    let mut existing = 0;

    for attachment in &page.attachments {
        let comments = if with_comments {
            client.attachment_comments(attachment).await.map_or_else(
                |_| Vec::new(),
                |comments| {
                    if !comments.is_empty() {
                        commented += 1;
                    }
                    comments
                },
            )
        } else {
            Vec::new()
        };
        items.push((attachment, comments));

        let dest = thread_dir.join(&attachment.name);
        if file_present(&dest, attachment.size.as_deref()) {
            existing += 1;
            continue;
        }
        match client.download_attachment_to(attachment, &dest).await {
            Ok(_) => {}
            Err(err) => println!("  skipped {} ({err})", attachment.name),
        }
    }

    if with_comments {
        let has_comments = items.iter().any(|(_, comments)| !comments.is_empty());
        if has_comments {
            std::fs::write(
                thread_dir.join("comments.toml"),
                write_comments_toml(&page.title, id, &items),
            )?;
        }
    }
    std::fs::write(
        thread_dir.join("manifest.toml"),
        write_manifest_toml(&page.title, id, &page.attachments),
    )?;

    Ok(Some((page.attachments.len(), existing, commented)))
}

fn file_present(path: &Path, size: Option<&str>) -> bool {
    let Ok(meta) = std::fs::metadata(path) else {
        return false;
    };
    size.and_then(parse_size_bytes)
        .is_none_or(|listed| meta.len() >= listed * 9 / 10)
}

fn parse_size_bytes(size: &str) -> Option<u64> {
    let mut parts = size.split_whitespace();
    let num: f64 = parts.next()?.parse().ok()?;
    match parts.next()? {
        "KB" => Some((num * 1024.0) as u64),
        "MB" => Some((num * 1024.0 * 1024.0) as u64),
        _ => Some(num as u64),
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    match args.command {
        Command::Login { login, password } => {
            let login = match login {
                Some(login) => login,
                None => prompt("Login: ")?,
            };
            let password = match password {
                Some(password) => password,
                None => rpassword::prompt_password("Password: ")?,
            };

            let mut client = FuoClient::builder()
                .timeout(Duration::from_secs(30))
                .build()?;
            match client.login(&login, &password, true).await {
                Ok(()) => {
                    save_session(&client)?;
                    println!("Authenticated: {}", client.is_logged_in());
                }
                Err(fuo::prelude::Error::AccountBanned) => {
                    println!(
                        "Account is locked: the site detected abnormal access (truy cập quá nhanh / tải nhiều ảnh)."
                    );
                    println!(
                        "Send an appeal if this is a mistake: https://fuoverflow.com/anti-crawl-appeal/"
                    );
                }
                Err(err) => return Err(err.into()),
            }
        }

        Command::Search {
            subject,
            filter,
            login,
            password,
        } => {
            let mut client = FuoClient::builder()
                .timeout(Duration::from_secs(30))
                .build()?;

            if let Some(cookies) = load_session()? {
                client.restore_cookies(&cookies);
            }

            if let (Some(login), Some(password)) = (login, password) {
                client.login(&login, &password, true).await?;
                save_session(&client)?;
            }

            let mut threads = client.search_subject(&subject).await?;
            if let Some(filter) = filter {
                let filter = filter.to_lowercase();
                threads.retain(|t| {
                    t.prefix
                        .as_deref()
                        .unwrap_or("")
                        .to_lowercase()
                        .contains(&filter)
                });
            }

            print_threads(&threads);
        }

        Command::Logout => {
            let path = session_path()?;
            if path.exists() {
                std::fs::remove_file(&path)?;
                println!("Logged out.");
            } else {
                println!("No saved session.");
            }
        }

        Command::Install {
            target,
            no_comments,
            dir,
            delay_ms,
        } => {
            let dir = expand_tilde(&dir);
            let client = FuoClient::builder()
                .timeout(Duration::from_secs(60))
                .delay(Duration::from_millis(delay_ms))
                .build()?;
            if let Some(cookies) = load_session()? {
                client.restore_cookies(&cookies);
            }
            if !client.is_logged_in() {
                println!(
                    "Warning: not logged in — comments and full-res downloads need a premium session (run `login` first)."
                );
            }
            std::fs::create_dir_all(&dir)?;

            let thread_id = target
                .rsplit_once('.')
                .and_then(|(_, id)| id.parse::<u32>().ok())
                .or_else(|| target.parse::<u32>().ok());

            if let Some(id) = thread_id {
                let start = std::time::Instant::now();
                println!("Installing thread {id}...");
                match install_thread(&client, id, &dir, !no_comments).await {
                    Ok(Some((attachments, existing, commented))) => println!(
                        "Done: {attachments} attachments ({existing} present, {commented} with comments) in {:.1}s -> {}",
                        start.elapsed().as_secs_f64(),
                        dir.display()
                    ),
                    Ok(None) => println!("Already installed -> {}", dir.display()),
                    Err(err) if is_forbidden(&err) => {
                        println!("Blocked by the site (403). Wait a while before retrying.");
                    }
                    Err(err) => println!("Failed: {err}"),
                }
            } else {
                let subject = target.to_uppercase();
                let threads = client.search_subject(&subject).await?;
                println!("Installing {} threads of {subject}...", threads.len());
                let subject_dir = dir.join(&subject);
                std::fs::create_dir_all(&subject_dir)?;

                let started = std::time::Instant::now();
                let mut done = 0;
                let mut skipped = 0;
                let mut failed = 0;

                for (i, thread) in threads.iter().enumerate() {
                    let start = std::time::Instant::now();
                    println!(
                        "[{}/{}] {} ({})",
                        i + 1,
                        threads.len(),
                        thread.id,
                        thread.title
                    );
                    match install_thread(&client, thread.id, &subject_dir, !no_comments).await {
                        Ok(Some((attachments, existing, commented))) => {
                            done += 1;
                            println!(
                                "  {} attachments ({existing} present, {commented} with comments) in {:.1}s",
                                attachments,
                                start.elapsed().as_secs_f64()
                            );
                        }
                        Ok(None) => {
                            skipped += 1;
                            println!("  already installed");
                        }
                        Err(err) if is_forbidden(&err) => {
                            println!(
                                "  stopped: blocked by the site (403). Wait a while before retrying."
                            );
                            break;
                        }
                        Err(err) => {
                            failed += 1;
                            println!("  skipped: {err}");
                        }
                    }
                    if i + 1 < threads.len() {
                        tokio::time::sleep(Duration::from_millis(200)).await;
                    }
                }

                println!(
                    "\nSummary: {}/{} threads done ({skipped} already installed, {failed} failed) in {:.0}s -> {}",
                    done,
                    threads.len(),
                    started.elapsed().as_secs_f64(),
                    subject_dir.display()
                );
            }
        }

        Command::Thread {
            id,
            download,
            comments,
            dir,
            delay_ms,
        } => {
            let dir = expand_tilde(&dir);
            let client = FuoClient::builder()
                .timeout(Duration::from_secs(60))
                .delay(Duration::from_millis(delay_ms))
                .build()?;
            if let Some(cookies) = load_session()? {
                client.restore_cookies(&cookies);
            }

            let page = client.thread_page(id).await?;
            println!("{}", page.title);
            println!();
            print_attachments(&page.attachments);

            if comments {
                for attachment in &page.attachments {
                    match client.attachment_comments(attachment).await {
                        Ok(comments) if !comments.is_empty() => {
                            println!("\n{} (attachment {})", attachment.name, attachment.id);
                            for comment in comments {
                                println!(
                                    "  {} ({}){}",
                                    comment.author,
                                    comment.date,
                                    vote_suffix(comment.vote.as_deref())
                                );
                                println!("    {}", comment.body.replace('\n', "\n    "));
                            }
                        }
                        Ok(_) => {}
                        Err(err) => {
                            println!(
                                "\n{} (attachment {}): skipped ({err})",
                                attachment.name, attachment.id
                            );
                        }
                    }
                }
            }

            if download {
                std::fs::create_dir_all(&dir)?;
                for attachment in &page.attachments {
                    let path = dir.join(&attachment.name);
                    match client.download_attachment_to(attachment, &path).await {
                        Ok(bytes) => println!("saved {} ({bytes} bytes)", path.display()),
                        Err(err) => {
                            println!("skipped {} ({err})", attachment.name);
                        }
                    }
                }
            }
        }
    }

    Ok(())
}
