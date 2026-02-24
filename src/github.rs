use base64::Engine;
use gloo_net::http::{Request, Response};
use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd};
use serde::Deserialize;
use std::{cell::RefCell, collections::HashMap};

const GITHUB_API_BASE: &str = "https://api.github.com";
const GITHUB_ACCEPT: &str = "application/vnd.github+json";
const GITHUB_API_VERSION: &str = "2022-11-28";
const GITHUB_USER_AGENT: &str = "objz-portfolio-terminal";
const CACHE_TTL_MS: f64 = 5.0 * 60.0 * 1000.0;

pub const DEFAULT_GITHUB_USER: &str = "objz";

#[derive(Debug, Clone)]
pub struct ProjectSummary {
    pub name: String,
    pub description: Option<String>,
    pub language: Option<String>,
    pub stars: u64,
    pub html_url: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheState {
    Live,
    Cached,
    StaleFallback,
}

#[derive(Debug, Clone)]
pub struct ProjectsSnapshot {
    pub username: String,
    pub projects: Vec<ProjectSummary>,
    pub fetched_at: f64,
    pub state: CacheState,
}

#[derive(Debug, Clone)]
pub struct ReadmeSnapshot {
    pub username: String,
    pub repo: String,
    pub content: String,
    pub fetched_at: f64,
    pub state: CacheState,
}

#[derive(Debug, Clone)]
struct CachedProjectsEntry {
    projects: Vec<ProjectSummary>,
    fetched_at: f64,
}

#[derive(Debug, Clone)]
struct CachedReadmeEntry {
    content: String,
    fetched_at: f64,
}

#[derive(Default)]
struct GithubCache {
    projects_by_user: HashMap<String, CachedProjectsEntry>,
    readmes_by_repo: HashMap<(String, String), CachedReadmeEntry>,
}

thread_local! {
    static CACHE: RefCell<GithubCache> = RefCell::new(GithubCache::default());
}

#[derive(Debug, Deserialize)]
struct GithubRepo {
    name: String,
    description: Option<String>,
    language: Option<String>,
    stargazers_count: u64,
    html_url: String,
    pushed_at: String,
    updated_at: String,
    fork: bool,
    archived: bool,
}

#[derive(Debug, Deserialize)]
struct GithubReadme {
    content: Option<String>,
    encoding: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GithubError {
    message: String,
}

pub async fn fetch_projects_cached(
    username: &str,
    force_refresh: bool,
) -> Result<ProjectsSnapshot, String> {
    let username = sanitize_segment(username);
    if username.is_empty() {
        return Err("username is required".into());
    }

    let now = js_sys::Date::now();

    if !force_refresh {
        if let Some(entry) = get_cached_projects(&username) {
            if is_fresh(now, entry.fetched_at) {
                return Ok(ProjectsSnapshot {
                    username,
                    projects: entry.projects,
                    fetched_at: entry.fetched_at,
                    state: CacheState::Cached,
                });
            }
        }
    }

    match fetch_projects(&username).await {
        Ok(projects) => {
            let fetched_at = js_sys::Date::now();
            set_cached_projects(&username, &projects, fetched_at);

            Ok(ProjectsSnapshot {
                username,
                projects,
                fetched_at,
                state: CacheState::Live,
            })
        }
        Err(error) => {
            if let Some(entry) = get_cached_projects(&username) {
                return Ok(ProjectsSnapshot {
                    username,
                    projects: entry.projects,
                    fetched_at: entry.fetched_at,
                    state: CacheState::StaleFallback,
                });
            }

            Err(error)
        }
    }
}

pub async fn fetch_readme_cached(
    username: &str,
    repo: &str,
    force_refresh: bool,
) -> Result<ReadmeSnapshot, String> {
    let username = sanitize_segment(username);
    let repo = sanitize_segment(repo);

    if username.is_empty() || repo.is_empty() {
        return Err("username and repo are required".into());
    }

    let now = js_sys::Date::now();
    if !force_refresh {
        if let Some(entry) = get_cached_readme(&username, &repo) {
            if is_fresh(now, entry.fetched_at) {
                return Ok(ReadmeSnapshot {
                    username,
                    repo,
                    content: entry.content,
                    fetched_at: entry.fetched_at,
                    state: CacheState::Cached,
                });
            }
        }
    }

    match fetch_readme(&username, &repo).await {
        Ok(content) => {
            let fetched_at = js_sys::Date::now();
            set_cached_readme(&username, &repo, &content, fetched_at);

            Ok(ReadmeSnapshot {
                username,
                repo,
                content,
                fetched_at,
                state: CacheState::Live,
            })
        }
        Err(error) => {
            if let Some(entry) = get_cached_readme(&username, &repo) {
                return Ok(ReadmeSnapshot {
                    username,
                    repo,
                    content: entry.content,
                    fetched_at: entry.fetched_at,
                    state: CacheState::StaleFallback,
                });
            }

            Err(error)
        }
    }
}

pub fn format_age(fetched_at: f64) -> String {
    let age_ms = (js_sys::Date::now() - fetched_at).max(0.0);
    let age_secs = (age_ms / 1000.0).round() as i64;

    if age_secs <= 1 {
        "just now".to_string()
    } else if age_secs < 60 {
        format!("{}s ago", age_secs)
    } else if age_secs < 3600 {
        format!("{}m ago", age_secs / 60)
    } else {
        format!("{}h ago", age_secs / 3600)
    }
}

pub fn render_markdown_to_terminal(markdown: &str) -> String {
    #[derive(Clone, Copy)]
    struct ListFrame {
        ordered: bool,
        next_index: u64,
    }

    let parser_options = Options::ENABLE_TABLES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_FOOTNOTES;
    let parser = Parser::new_ext(markdown, parser_options);

    let mut output = String::new();
    let mut list_stack: Vec<ListFrame> = Vec::new();
    let mut link_stack: Vec<(String, String)> = Vec::new();
    let mut in_code_block = false;
    let mut in_blockquote = false;
    let mut in_heading = false;
    let mut table_cell_index: usize = 0;

    let push_text = |text: &str, output: &mut String, link_stack: &mut Vec<(String, String)>| {
        if let Some((_, link_text)) = link_stack.last_mut() {
            link_text.push_str(text);
        } else {
            output.push_str(text);
        }
    };

    for event in parser {
        match event {
            Event::Start(tag) => match tag {
                Tag::Heading { level, .. } => {
                    if !output.ends_with("\n\n") && !output.is_empty() {
                        output.push_str("\n\n");
                    }

                    in_heading = true;
                    let heading_color = match level as usize {
                        1 => "\x1b[1;96m",
                        2 => "\x1b[1;36m",
                        _ => "\x1b[36m",
                    };
                    output.push_str(heading_color);
                }
                Tag::Paragraph => {
                    if !output.is_empty() && !output.ends_with("\n\n") {
                        output.push_str("\n\n");
                    }
                }
                Tag::List(start) => {
                    list_stack.push(ListFrame {
                        ordered: start.is_some(),
                        next_index: start.unwrap_or(1),
                    });
                    if !output.ends_with('\n') {
                        output.push('\n');
                    }
                }
                Tag::Item => {
                    if !output.ends_with('\n') {
                        output.push('\n');
                    }

                    let indent_depth = list_stack.len().saturating_sub(1);
                    output.push_str(&"  ".repeat(indent_depth));

                    if let Some(frame) = list_stack.last_mut() {
                        if frame.ordered {
                            output.push_str(&format!("{}. ", frame.next_index));
                            frame.next_index += 1;
                        } else {
                            output.push_str("- ");
                        }
                    }
                }
                Tag::BlockQuote(_) => {
                    if !output.ends_with('\n') {
                        output.push('\n');
                    }
                    in_blockquote = true;
                    output.push_str("\x1b[90m| \x1b[0m");
                }
                Tag::CodeBlock(kind) => {
                    if !output.ends_with("\n\n") && !output.is_empty() {
                        output.push_str("\n\n");
                    }
                    in_code_block = true;

                    match kind {
                        CodeBlockKind::Fenced(language) if !language.is_empty() => {
                            output.push_str(&format!("\x1b[90m[code:{}]\x1b[0m\n", language));
                        }
                        _ => output.push_str("\x1b[90m[code]\x1b[0m\n"),
                    }
                }
                Tag::Link { dest_url, .. } => {
                    link_stack.push((dest_url.to_string(), String::new()));
                }
                Tag::Table(_) => {
                    if !output.ends_with("\n\n") && !output.is_empty() {
                        output.push_str("\n\n");
                    }
                }
                Tag::TableRow => {
                    if !output.ends_with('\n') && !output.is_empty() {
                        output.push('\n');
                    }
                    table_cell_index = 0;
                }
                Tag::TableCell => {
                    if table_cell_index > 0 {
                        output.push_str(" | ");
                    }
                }
                _ => {}
            },
            Event::End(tag) => match tag {
                TagEnd::Heading(_) => {
                    if in_heading {
                        output.push_str("\x1b[0m");
                        in_heading = false;
                    }
                    output.push_str("\n\n");
                }
                TagEnd::Paragraph => {
                    output.push_str("\n\n");
                }
                TagEnd::List(_) => {
                    list_stack.pop();
                    if !output.ends_with("\n\n") {
                        output.push('\n');
                    }
                }
                TagEnd::Item => {
                    if !output.ends_with('\n') {
                        output.push('\n');
                    }
                }
                TagEnd::CodeBlock => {
                    in_code_block = false;
                    if !output.ends_with('\n') {
                        output.push('\n');
                    }
                    output.push('\n');
                }
                TagEnd::BlockQuote(_) => {
                    in_blockquote = false;
                    if !output.ends_with("\n\n") {
                        output.push('\n');
                    }
                }
                TagEnd::Link => {
                    if let Some((url, text)) = link_stack.pop() {
                        if text.trim().is_empty() {
                            output.push_str(&url);
                        } else {
                            output.push_str(&format!("{} ({})", text.trim(), url));
                        }
                    }
                }
                TagEnd::Table => {
                    if !output.ends_with("\n\n") {
                        output.push_str("\n\n");
                    }
                }
                TagEnd::TableRow => {
                    output.push('\n');
                }
                TagEnd::TableCell => {
                    table_cell_index += 1;
                }
                _ => {}
            },
            Event::Text(text) => {
                if in_code_block {
                    for (index, line) in text.lines().enumerate() {
                        if index > 0 {
                            output.push('\n');
                        }
                        output.push_str("    ");
                        output.push_str(line);
                    }
                } else {
                    push_text(&text, &mut output, &mut link_stack)
                }
            }
            Event::Code(code) => {
                let code_text = format!("`{}`", code);
                push_text(&code_text, &mut output, &mut link_stack);
            }
            Event::SoftBreak => {
                if in_code_block {
                    output.push('\n');
                    output.push_str("    ");
                } else {
                    output.push(' ');
                }
            }
            Event::HardBreak => {
                output.push('\n');
                if in_blockquote {
                    output.push_str("\x1b[90m| \x1b[0m");
                }
            }
            Event::Rule => {
                if !output.ends_with('\n') {
                    output.push('\n');
                }
                output.push_str("---\n");
            }
            Event::Html(html) => {
                let text = html.replace(['<', '>'], " ");
                push_text(text.trim(), &mut output, &mut link_stack);
            }
            Event::InlineHtml(html) => {
                let text = html.replace(['<', '>'], " ");
                push_text(text.trim(), &mut output, &mut link_stack);
            }
            Event::FootnoteReference(name) => {
                push_text(&format!("[{}]", name), &mut output, &mut link_stack);
            }
            Event::TaskListMarker(checked) => {
                push_text(
                    if checked { "[x] " } else { "[ ] " },
                    &mut output,
                    &mut link_stack,
                );
            }
            _ => {}
        }
    }

    while output.contains("\n\n\n") {
        output = output.replace("\n\n\n", "\n\n");
    }

    output.trim_end().to_string()
}

async fn fetch_projects(username: &str) -> Result<Vec<ProjectSummary>, String> {
    let url = format!(
        "{}/users/{}/repos?type=public&sort=updated&per_page=100",
        GITHUB_API_BASE, username
    );

    let response = github_get(&url).await?;
    let mut repos: Vec<GithubRepo> = response
        .json()
        .await
        .map_err(|error| format!("failed to parse repository list: {}", error))?;

    repos.retain(|repo| !repo.fork && !repo.archived);
    repos.sort_by(|a, b| b.pushed_at.cmp(&a.pushed_at));

    Ok(repos
        .into_iter()
        .map(|repo| ProjectSummary {
            name: repo.name,
            description: repo.description,
            language: repo.language,
            stars: repo.stargazers_count,
            html_url: repo.html_url,
            updated_at: repo.updated_at,
        })
        .collect())
}

async fn fetch_readme(username: &str, repo: &str) -> Result<String, String> {
    let url = format!("{}/repos/{}/{}/readme", GITHUB_API_BASE, username, repo);
    let response = github_get(&url).await?;
    let readme: GithubReadme = response
        .json()
        .await
        .map_err(|error| format!("failed to parse readme response: {}", error))?;

    let content = readme
        .content
        .ok_or_else(|| "readme content was missing".to_string())?;

    let encoding = readme
        .encoding
        .unwrap_or_else(|| "base64".to_string())
        .to_lowercase();

    if encoding != "base64" {
        return Err(format!("unsupported readme encoding: {}", encoding));
    }

    let compact_base64 = content.replace(['\r', '\n'], "");
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(compact_base64)
        .map_err(|error| format!("failed to decode readme: {}", error))?;

    String::from_utf8(decoded).map_err(|error| format!("readme was not valid utf-8: {}", error))
}

async fn github_get(url: &str) -> Result<Response, String> {
    let response = Request::get(url)
        .header("Accept", GITHUB_ACCEPT)
        .header("X-GitHub-Api-Version", GITHUB_API_VERSION)
        .header("User-Agent", GITHUB_USER_AGENT)
        .send()
        .await
        .map_err(|error| format!("network error while calling GitHub: {}", error))?;

    if response.ok() {
        Ok(response)
    } else {
        Err(read_github_error(response).await)
    }
}

async fn read_github_error(response: Response) -> String {
    let status = response.status();
    let body = response.text().await.unwrap_or_default();

    if body.trim().is_empty() {
        return format!("GitHub API returned status {}", status);
    }

    if let Ok(parsed) = serde_json::from_str::<GithubError>(&body) {
        return format!("GitHub API {}: {}", status, parsed.message);
    }

    format!("GitHub API {}: {}", status, body)
}

fn is_fresh(now: f64, fetched_at: f64) -> bool {
    now - fetched_at <= CACHE_TTL_MS
}

fn get_cached_projects(username: &str) -> Option<CachedProjectsEntry> {
    CACHE.with(|cache| cache.borrow().projects_by_user.get(username).cloned())
}

fn set_cached_projects(username: &str, projects: &[ProjectSummary], fetched_at: f64) {
    CACHE.with(|cache| {
        cache.borrow_mut().projects_by_user.insert(
            username.to_string(),
            CachedProjectsEntry {
                projects: projects.to_vec(),
                fetched_at,
            },
        );
    });
}

fn get_cached_readme(username: &str, repo: &str) -> Option<CachedReadmeEntry> {
    let key = (username.to_string(), repo.to_string());
    CACHE.with(|cache| cache.borrow().readmes_by_repo.get(&key).cloned())
}

fn set_cached_readme(username: &str, repo: &str, content: &str, fetched_at: f64) {
    CACHE.with(|cache| {
        cache.borrow_mut().readmes_by_repo.insert(
            (username.to_string(), repo.to_string()),
            CachedReadmeEntry {
                content: content.to_string(),
                fetched_at,
            },
        );
    });
}

fn sanitize_segment(value: &str) -> String {
    value
        .trim()
        .trim_matches('/')
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.'))
        .collect()
}
