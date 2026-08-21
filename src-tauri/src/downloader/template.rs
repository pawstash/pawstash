use std::path::Path;

#[derive(Debug, Clone)]
pub struct TemplateContext<'a> {
    pub service: &'a str,
    pub creator_id: &'a str,
    pub creator_name: &'a str,
    pub post_id: &'a str,
    pub post_title: &'a str,
    pub published: Option<&'a str>,
    pub original_filename: &'a str,
    pub index: usize,
    pub media_id: &'a str,
}

pub fn sanitize_path_segment(input: &str, max_len: usize) -> String {
    let mut sanitized = String::with_capacity(input.len());
    for c in input.chars() {
        match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => {
                sanitized.push('_');
            }
            c if (c as u32) <= 31 => {
                sanitized.push('_');
            }
            other => sanitized.push(other),
        }
    }

    // Collapse multiple underscores/spaces
    let mut cleaned = String::with_capacity(sanitized.len());
    let mut prev_char: Option<char> = None;
    for c in sanitized.chars() {
        if (c == '_' || c == ' ') && prev_char == Some(c) {
            continue;
        }
        cleaned.push(c);
        prev_char = Some(c);
    }

    // Trim trailing and leading whitespace and dots (Windows requirement)
    let trimmed = cleaned.trim().trim_matches('.').trim();

    // Check Windows reserved names
    let upper = trimmed.to_ascii_uppercase();
    let is_reserved = matches!(
        upper.as_str(),
        "CON"
            | "PRN"
            | "AUX"
            | "NUL"
            | "COM1"
            | "COM2"
            | "COM3"
            | "COM4"
            | "COM5"
            | "COM6"
            | "COM7"
            | "COM8"
            | "COM9"
            | "LPT1"
            | "LPT2"
            | "LPT3"
            | "LPT4"
            | "LPT5"
            | "LPT6"
            | "LPT7"
            | "LPT8"
            | "LPT9"
    );

    let base = if is_reserved {
        format!("_{trimmed}")
    } else if trimmed.is_empty() {
        "unnamed".to_string()
    } else {
        trimmed.to_string()
    };

    // Safe UTF-8 truncation
    if base.chars().count() > max_len {
        let truncated: String = base.chars().take(max_len).collect();
        let trimmed_trunc = truncated.trim().trim_matches('.').trim();
        if trimmed_trunc.is_empty() {
            "unnamed".to_string()
        } else {
            trimmed_trunc.to_string()
        }
    } else {
        base
    }
}

#[derive(Debug, Clone)]
struct DateParts {
    date_iso: String,
    date_compact: String,
    date_dots: String,
    year: String,
    year_short: String,
    month: String,
    day: String,
    year_month: String,
}

fn parse_date_parts(published: Option<&str>) -> DateParts {
    let raw = published.unwrap_or("").trim();
    let (year, month, day) = if raw.len() >= 10 && &raw[4..5] == "-" && &raw[7..8] == "-" {
        (raw[0..4].to_string(), raw[5..7].to_string(), raw[8..10].to_string())
    } else if raw.len() >= 10 && &raw[4..5] == "/" && &raw[7..8] == "/" {
        (raw[0..4].to_string(), raw[5..7].to_string(), raw[8..10].to_string())
    } else if raw.len() >= 10 && &raw[2..3] == "." && &raw[5..6] == "." {
        (raw[6..10].to_string(), raw[3..5].to_string(), raw[0..2].to_string())
    } else {
        ("".to_string(), "".to_string(), "".to_string())
    };

    if !year.is_empty() && !month.is_empty() && !day.is_empty() {
        let year_short = if year.len() >= 4 { year[2..4].to_string() } else { year.clone() };
        DateParts {
            date_iso: format!("{year}-{month}-{day}"),
            date_compact: format!("{year}{month}{day}"),
            date_dots: format!("{year}.{month}.{day}"),
            year: year.clone(),
            year_short,
            month: month.clone(),
            day: day.clone(),
            year_month: format!("{year}-{month}"),
        }
    } else {
        DateParts {
            date_iso: raw.to_string(),
            date_compact: raw.replace('-', "").replace('.', "").replace('/', ""),
            date_dots: raw.replace('-', ".").replace('/', "."),
            year: String::new(),
            year_short: String::new(),
            month: String::new(),
            day: String::new(),
            year_month: String::new(),
        }
    }
}

pub fn resolve_creator_folder(template: &str, ctx: &TemplateContext) -> String {
    let tpl = if template.trim().is_empty() {
        "{creator}"
    } else {
        template
    };

    let author_name = if !ctx.creator_name.trim().is_empty() {
        ctx.creator_name.trim()
    } else {
        ctx.creator_id.trim()
    };

    let date = parse_date_parts(ctx.published);

    let mut result = tpl.to_string();
    result = result.replace("{creator_id}", ctx.creator_id);
    result = result.replace("{creator}", author_name);
    result = result.replace("{author}", author_name);
    result = result.replace("{name}", author_name);
    result = result.replace("{id}", ctx.creator_id);
    result = result.replace("{service}", ctx.service);
    result = result.replace("{platform}", ctx.service);
    result = result.replace("{date_compact}", &date.date_compact);
    result = result.replace("{date_dots}", &date.date_dots);
    result = result.replace("{date}", &date.date_iso);
    result = result.replace("{published}", &date.date_iso);
    result = result.replace("{year_short}", &date.year_short);
    result = result.replace("{year_month}", &date.year_month);
    result = result.replace("{year}", &date.year);
    result = result.replace("{yyyy}", &date.year);
    result = result.replace("{yy}", &date.year_short);
    result = result.replace("{month}", &date.month);
    result = result.replace("{mm}", &date.month);
    result = result.replace("{day}", &date.day);
    result = result.replace("{dd}", &date.day);

    sanitize_path_segment(&result, 120)
}

pub fn resolve_post_folder(template: &str, ctx: &TemplateContext) -> String {
    let tpl = if template.trim().is_empty() {
        "{post_title}"
    } else {
        template
    };

    let title = if !ctx.post_title.trim().is_empty() {
        ctx.post_title.trim()
    } else {
        ctx.post_id.trim()
    };

    let author_name = if !ctx.creator_name.trim().is_empty() {
        ctx.creator_name.trim()
    } else {
        ctx.creator_id.trim()
    };

    let date = parse_date_parts(ctx.published);

    let mut result = tpl.to_string();
    result = result.replace("{post_title}", title);
    result = result.replace("{title}", title);
    result = result.replace("{post_id}", ctx.post_id);
    result = result.replace("{creator_id}", ctx.creator_id);
    result = result.replace("{creator}", author_name);
    result = result.replace("{author}", author_name);
    result = result.replace("{service}", ctx.service);
    result = result.replace("{platform}", ctx.service);
    result = result.replace("{date_compact}", &date.date_compact);
    result = result.replace("{date_dots}", &date.date_dots);
    result = result.replace("{date}", &date.date_iso);
    result = result.replace("{published}", &date.date_iso);
    result = result.replace("{year_short}", &date.year_short);
    result = result.replace("{year_month}", &date.year_month);
    result = result.replace("{year}", &date.year);
    result = result.replace("{yyyy}", &date.year);
    result = result.replace("{yy}", &date.year_short);
    result = result.replace("{month}", &date.month);
    result = result.replace("{mm}", &date.month);
    result = result.replace("{day}", &date.day);
    result = result.replace("{dd}", &date.day);
    result = result.replace("{id}", ctx.post_id);

    sanitize_path_segment(&result, 120)
}

pub fn resolve_filename(template: &str, ctx: &TemplateContext) -> String {
    let tpl = if template.trim().is_empty() {
        "{post_title} - {filename}"
    } else {
        template
    };

    let orig_path = Path::new(ctx.original_filename);
    let orig_stem = orig_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("file");
    let orig_ext = orig_path.extension().and_then(|e| e.to_str()).unwrap_or("");

    let title = if !ctx.post_title.trim().is_empty() {
        ctx.post_title.trim()
    } else {
        ctx.post_id.trim()
    };

    let author_name = if !ctx.creator_name.trim().is_empty() {
        ctx.creator_name.trim()
    } else {
        ctx.creator_id.trim()
    };

    let date = parse_date_parts(ctx.published);

    let mut result = tpl.to_string();

    // Longer tokens first to prevent partial token replacement collisions
    result = result.replace("{post_title}", title);
    result = result.replace("{title}", title);
    result = result.replace("{post_id}", ctx.post_id);

    result = result.replace("{creator_id}", ctx.creator_id);
    result = result.replace("{creator}", author_name);
    result = result.replace("{author}", author_name);

    result = result.replace("{service}", ctx.service);
    result = result.replace("{platform}", ctx.service);

    result = result.replace("{original_name}", ctx.original_filename);
    result = result.replace("{filename}", ctx.original_filename);
    result = result.replace("{name}", orig_stem);
    result = result.replace("{ext}", orig_ext);
    result = result.replace("{index}", &ctx.index.to_string());

    result = result.replace("{date_compact}", &date.date_compact);
    result = result.replace("{date_dots}", &date.date_dots);
    result = result.replace("{date}", &date.date_iso);
    result = result.replace("{published}", &date.date_iso);

    result = result.replace("{year_short}", &date.year_short);
    result = result.replace("{year_month}", &date.year_month);
    result = result.replace("{year}", &date.year);
    result = result.replace("{yyyy}", &date.year);
    result = result.replace("{yy}", &date.year_short);

    result = result.replace("{month}", &date.month);
    result = result.replace("{mm}", &date.month);
    result = result.replace("{day}", &date.day);
    result = result.replace("{dd}", &date.day);

    result = result.replace("{media_id}", ctx.media_id);
    result = result.replace("{id}", ctx.post_id);

    // If result ends with .<orig_ext> (e.g. user wrote .png or .{ext} or {filename} was at end), strip it for clean stem sanitization
    let mut stem = result.trim().to_string();
    if !orig_ext.is_empty() {
        let dot_ext = format!(".{}", orig_ext.to_ascii_lowercase());
        if stem.to_ascii_lowercase().ends_with(&dot_ext) {
            stem.truncate(stem.len() - dot_ext.len());
        }
    }

    let sanitized_stem = sanitize_path_segment(&stem, 180);
    let sanitized_ext = sanitize_path_segment(orig_ext, 16);

    if sanitized_ext.is_empty() {
        sanitized_stem
    } else {
        format!("{sanitized_stem}.{sanitized_ext}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitization() {
        assert_eq!(sanitize_path_segment("foo/bar:baz*qux", 100), "foo_bar_baz_qux");
        assert_eq!(sanitize_path_segment("  hello world...  ", 100), "hello world");
        assert_eq!(sanitize_path_segment("CON", 100), "_CON");
        assert_eq!(sanitize_path_segment("aux", 100), "_aux");
    }

    #[test]
    fn test_creator_folder_resolution() {
        let ctx = TemplateContext {
            service: "fanbox",
            creator_id: "12345",
            creator_name: "haku3490",
            post_id: "999",
            post_title: "My Post",
            published: Some("2024-08-20T12:00:00Z"),
            original_filename: "image.png",
            index: 1,
            media_id: "file1",
        };
        assert_eq!(resolve_creator_folder("{creator}", &ctx), "haku3490");
        assert_eq!(
            resolve_creator_folder("{creator} ({platform})", &ctx),
            "haku3490 (fanbox)"
        );
    }

    #[test]
    fn test_post_folder_resolution() {
        let ctx = TemplateContext {
            service: "fanbox",
            creator_id: "12345",
            creator_name: "haku3490",
            post_id: "999",
            post_title: "Summer Beach 2024",
            published: Some("2024-08-20T12:00:00Z"),
            original_filename: "image.png",
            index: 1,
            media_id: "file1",
        };
        assert_eq!(resolve_post_folder("{post_title}", &ctx), "Summer Beach 2024");
        assert_eq!(
            resolve_post_folder("[{date}] {post_id} - {title}", &ctx),
            "[2024-08-20] 999 - Summer Beach 2024"
        );
    }

    #[test]
    fn test_filename_resolution() {
        let ctx = TemplateContext {
            service: "patreon",
            creator_id: "12345",
            creator_name: "Artist",
            post_id: "888",
            post_title: "New Artwork",
            published: Some("2024-08-20"),
            original_filename: "0u5Og13y8DV.png",
            index: 2,
            media_id: "0u5Og13y8DV.png",
        };
        assert_eq!(
            resolve_filename("{post_title} - {filename}", &ctx),
            "New Artwork - 0u5Og13y8DV.png"
        );
        assert_eq!(
            resolve_filename("{post_title} - {index}", &ctx),
            "New Artwork - 2.png"
        );
        assert_eq!(
            resolve_filename("{creator} - {post_id}_{name}.{ext}", &ctx),
            "Artist - 888_0u5Og13y8DV.png"
        );
        assert_eq!(
            resolve_filename("{year}/{year_month}/{date_compact}_{index}", &ctx),
            "2024_2024-08_20240820_2.png"
        );
        assert_eq!(
            resolve_filename("[{date_dots}] {post_title}_{year_short}", &ctx),
            "[2024.08.20] New Artwork_24.png"
        );
        assert_eq!(
            resolve_filename("{post_title} - {day}_{month}_{year}", &ctx),
            "New Artwork - 20_08_2024.png"
        );
    }
}
