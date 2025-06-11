use js_sys::Array;
use std::collections::HashMap;
use web_sys::console::log;

#[derive(Debug, Clone)]
pub struct LinkInfo {
    pub url: String,
    pub start_x: f64,
    pub end_x: f64,
    pub y: f64,
    pub line_height: f64,
}

pub struct LinkMap {
    links: HashMap<u32, Vec<LinkInfo>>,
    next_id: u32,
}

impl LinkMap {
    pub fn new() -> Self {
        Self {
            links: HashMap::new(),
            next_id: 0,
        }
    }

    pub fn detect_links(
        &mut self,
        text: &str,
        x: f64,
        y: f64,
        char_width: f64,
        line_height: f64,
    ) -> u32 {
        let line_id = self.next_id;
        self.next_id += 1;

        let mut links = Vec::new();
        let mut start = 0;

        while let Some(http_pos) = text[start..].find("http") {
            let url_start = start + http_pos;
            let remaining = &text[url_start..];

            if let Some(end_pos) =
                remaining.find(|c: char| c.is_whitespace() || c == '\n' || c == ']')
            {
                let url = &remaining[..end_pos];
                if url.starts_with("http://") || url.starts_with("https://") {
                    let start_x = x + (url_start as f64 * char_width);
                    let end_x = start_x + (url.len() as f64 * char_width);

                    log(&Array::of1(
                        &format!(
                            "Detected link: {} at ({}, {}) to ({}, {})",
                            url,
                            start_x,
                            y,
                            end_x,
                            y + line_height
                        )
                        .into(),
                    ));

                    links.push(LinkInfo {
                        url: url.to_string(),
                        start_x,
                        end_x,
                        y,
                        line_height,
                    });
                }
                start = url_start + end_pos;
            } else {
                let url = remaining;
                if url.starts_with("http://") || url.starts_with("https://") {
                    let start_x = x + (url_start as f64 * char_width);
                    let end_x = start_x + (url.len() as f64 * char_width);

                    log(&Array::of1(
                        &format!(
                            "Detected link: {} at ({}, {}) to ({}, {})",
                            url,
                            start_x,
                            y,
                            end_x,
                            y + line_height
                        )
                        .into(),
                    ));

                    links.push(LinkInfo {
                        url: url.to_string(),
                        start_x,
                        end_x,
                        y,
                        line_height,
                    });
                }
                break;
            }
        }

        if !links.is_empty() {
            self.links.insert(line_id, links);
        }

        line_id
    }

    pub fn find_link(&self, x: f64, y: f64) -> Option<String> {
        for links in self.links.values() {
            for link in links {
                if x >= link.start_x
                    && x <= link.end_x
                    && y >= link.y
                    && y <= link.y + link.line_height
                {
                    log(&Array::of2(
                        &"MATCH found:".into(),
                        &link.url.clone().into(),
                    ));
                    return Some(link.url.clone());
                }
            }
        }
        None
    }

    pub fn clear(&mut self) {
        self.links.clear();
        self.next_id = 0;
    }
}
