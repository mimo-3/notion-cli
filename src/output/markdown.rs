use std::io::Write;

use serde_json::Value;

use crate::error::CliError;

/// Convert an array of Notion blocks to Markdown.
pub fn write_markdown(value: &Value, writer: &mut dyn Write) -> Result<(), CliError> {
    match value {
        Value::Array(blocks) => {
            let mut ctx = RenderContext::new();
            render_blocks(blocks, 0, &mut ctx, writer)?;
        }
        Value::Object(map) => {
            // Single block
            let mut ctx = RenderContext::new();
            render_block(value, 0, &mut ctx, writer)?;
            // If it's a Notion list response, extract results
            if let Some(results) = map.get("results").and_then(|v| v.as_array()) {
                render_blocks(results, 0, &mut ctx, writer)?;
            }
        }
        Value::String(s) => {
            // Already markdown text
            write!(writer, "{s}")?;
        }
        _ => {
            writeln!(writer, "{value}")?;
        }
    }
    Ok(())
}

struct RenderContext {
    numbered_list_counter: u32,
    last_was_numbered: bool,
}

impl RenderContext {
    fn new() -> Self {
        Self {
            numbered_list_counter: 0,
            last_was_numbered: false,
        }
    }
}

fn render_blocks(
    blocks: &[Value],
    depth: usize,
    ctx: &mut RenderContext,
    writer: &mut dyn Write,
) -> Result<(), CliError> {
    for block in blocks {
        let block_type = block.get("type").and_then(|v| v.as_str()).unwrap_or("");
        if block_type != "numbered_list_item" && ctx.last_was_numbered {
            ctx.numbered_list_counter = 0;
            ctx.last_was_numbered = false;
        }
        render_block(block, depth, ctx, writer)?;
    }
    Ok(())
}

fn indent(depth: usize) -> String {
    "  ".repeat(depth)
}

fn render_block(
    block: &Value,
    depth: usize,
    ctx: &mut RenderContext,
    writer: &mut dyn Write,
) -> Result<(), CliError> {
    let block_type = block.get("type").and_then(|v| v.as_str()).unwrap_or("");
    let prefix = indent(depth);
    let type_data = block.get(block_type);

    match block_type {
        "paragraph" => {
            let text = extract_rich_text(type_data);
            writeln!(writer, "{prefix}{text}")?;
            writeln!(writer)?;
        }
        "heading_1" => {
            let text = extract_rich_text(type_data);
            writeln!(writer, "{prefix}# {text}")?;
            writeln!(writer)?;
        }
        "heading_2" => {
            let text = extract_rich_text(type_data);
            writeln!(writer, "{prefix}## {text}")?;
            writeln!(writer)?;
        }
        "heading_3" => {
            let text = extract_rich_text(type_data);
            writeln!(writer, "{prefix}### {text}")?;
            writeln!(writer)?;
        }
        "heading_4" => {
            let text = extract_rich_text(type_data);
            writeln!(writer, "{prefix}#### {text}")?;
            writeln!(writer)?;
        }
        "bulleted_list_item" => {
            let text = extract_rich_text(type_data);
            writeln!(writer, "{prefix}- {text}")?;
            render_children(block, block_type, depth + 1, ctx, writer)?;
        }
        "numbered_list_item" => {
            ctx.numbered_list_counter += 1;
            ctx.last_was_numbered = true;
            let text = extract_rich_text(type_data);
            writeln!(writer, "{prefix}{}. {text}", ctx.numbered_list_counter)?;
            render_children(block, block_type, depth + 1, ctx, writer)?;
        }
        "to_do" => {
            let text = extract_rich_text(type_data);
            let checked = type_data
                .and_then(|d| d.get("checked"))
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let mark = if checked { "x" } else { " " };
            writeln!(writer, "{prefix}- [{mark}] {text}")?;
            render_children(block, block_type, depth + 1, ctx, writer)?;
        }
        "toggle" => {
            let text = extract_rich_text(type_data);
            writeln!(writer, "{prefix}<details>")?;
            writeln!(writer, "{prefix}<summary>{text}</summary>")?;
            writeln!(writer)?;
            render_children(block, block_type, depth, ctx, writer)?;
            writeln!(writer, "{prefix}</details>")?;
            writeln!(writer)?;
        }
        "quote" => {
            let text = extract_rich_text(type_data);
            for line in text.lines() {
                writeln!(writer, "{prefix}> {line}")?;
            }
            writeln!(writer)?;
        }
        "callout" => {
            let text = extract_rich_text(type_data);
            let icon = type_data
                .and_then(|d| d.get("icon"))
                .and_then(|i| {
                    i.get("emoji")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                })
                .unwrap_or_default();
            let icon_prefix = if icon.is_empty() {
                String::new()
            } else {
                format!("{icon} ")
            };
            for line in text.lines() {
                writeln!(writer, "{prefix}> {icon_prefix}{line}")?;
            }
            writeln!(writer)?;
        }
        "code" => {
            let text = extract_rich_text(type_data);
            let language = type_data
                .and_then(|d| d.get("language"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            writeln!(writer, "{prefix}```{language}")?;
            for line in text.lines() {
                writeln!(writer, "{prefix}{line}")?;
            }
            writeln!(writer, "{prefix}```")?;
            writeln!(writer)?;
        }
        "equation" => {
            let expression = type_data
                .and_then(|d| d.get("expression"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            writeln!(writer, "{prefix}$$ {expression} $$")?;
            writeln!(writer)?;
        }
        "divider" => {
            writeln!(writer, "{prefix}---")?;
            writeln!(writer)?;
        }
        "image" => {
            let caption = extract_caption(type_data);
            let url = extract_file_url(type_data);
            writeln!(writer, "{prefix}![{caption}]({url})")?;
            writeln!(writer)?;
        }
        "video" => {
            let url = extract_file_url(type_data);
            writeln!(writer, "{prefix}[Video]({url})")?;
            writeln!(writer)?;
        }
        "file" => {
            let url = extract_file_url(type_data);
            let name = type_data
                .and_then(|d| d.get("name"))
                .and_then(|v| v.as_str())
                .unwrap_or("File");
            writeln!(writer, "{prefix}[{name}]({url})")?;
            writeln!(writer)?;
        }
        "bookmark" => {
            let url = type_data
                .and_then(|d| d.get("url"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let caption = extract_caption(type_data);
            let label = if caption.is_empty() {
                "Bookmark".to_string()
            } else {
                caption
            };
            writeln!(writer, "{prefix}[{label}]({url})")?;
            writeln!(writer)?;
        }
        "link_preview" => {
            let url = type_data
                .and_then(|d| d.get("url"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            writeln!(writer, "{prefix}[Link]({url})")?;
            writeln!(writer)?;
        }
        "table" => {
            // Table children (table_row) should be in the block's children
            render_children(block, block_type, depth, ctx, writer)?;
            writeln!(writer)?;
        }
        "table_row" => {
            if let Some(cells) = type_data.and_then(|d| d.get("cells")).and_then(|v| v.as_array())
            {
                let cell_texts: Vec<String> = cells
                    .iter()
                    .map(|cell| {
                        if let Some(arr) = cell.as_array() {
                            rich_text_to_markdown(arr)
                        } else {
                            String::new()
                        }
                    })
                    .collect();
                writeln!(writer, "{prefix}| {} |", cell_texts.join(" | "))?;
            }
        }
        "child_page" => {
            let title = type_data
                .and_then(|d| d.get("title"))
                .and_then(|v| v.as_str())
                .unwrap_or("Untitled");
            writeln!(writer, "{prefix}\u{1f4c4} {title}")?;
            writeln!(writer)?;
        }
        "child_database" => {
            let title = type_data
                .and_then(|d| d.get("title"))
                .and_then(|v| v.as_str())
                .unwrap_or("Untitled");
            writeln!(writer, "{prefix}\u{1f4ca} {title}")?;
            writeln!(writer)?;
        }
        _ => {
            // Fallback: try to extract rich text
            let text = extract_rich_text(type_data);
            if !text.is_empty() {
                writeln!(writer, "{prefix}{text}")?;
                writeln!(writer)?;
            }
        }
    }
    Ok(())
}

fn render_children(
    block: &Value,
    block_type: &str,
    depth: usize,
    ctx: &mut RenderContext,
    writer: &mut dyn Write,
) -> Result<(), CliError> {
    // Children may be embedded in the block type data
    if let Some(children) = block
        .get(block_type)
        .and_then(|d| d.get("children"))
        .and_then(|v| v.as_array())
    {
        render_blocks(children, depth, ctx, writer)?;
    }
    Ok(())
}

fn extract_rich_text(type_data: Option<&Value>) -> String {
    type_data
        .and_then(|d| d.get("rich_text"))
        .and_then(|v| v.as_array())
        .map(|arr| rich_text_to_markdown(arr))
        .unwrap_or_default()
}

fn rich_text_to_markdown(rich_text: &[Value]) -> String {
    let mut result = String::new();
    for rt in rich_text {
        let plain = rt.get("plain_text").and_then(|v| v.as_str()).unwrap_or("");
        let annotations = rt.get("annotations");
        let href = rt.get("href").and_then(|v| v.as_str());
        let rt_type = rt.get("type").and_then(|v| v.as_str()).unwrap_or("text");

        let mut text = plain.to_string();

        // Handle equation inline
        if rt_type == "equation" {
            if let Some(expr) = rt
                .get("equation")
                .and_then(|e| e.get("expression"))
                .and_then(|v| v.as_str())
            {
                text = format!("${expr}$");
                result.push_str(&text);
                continue;
            }
        }

        // Handle mention
        if rt_type == "mention" {
            if let Some(mention) = rt.get("mention") {
                let mention_type = mention.get("type").and_then(|v| v.as_str()).unwrap_or("");
                match mention_type {
                    "page" => {
                        text = format!("[[{plain}]]");
                    }
                    "user" => {
                        text = format!("@{plain}");
                    }
                    _ => {
                        text = format!("@{plain}");
                    }
                }
                result.push_str(&text);
                continue;
            }
        }

        // Apply annotations
        if let Some(ann) = annotations {
            let code = ann.get("code").and_then(|v| v.as_bool()).unwrap_or(false);
            let bold = ann.get("bold").and_then(|v| v.as_bool()).unwrap_or(false);
            let italic = ann.get("italic").and_then(|v| v.as_bool()).unwrap_or(false);
            let strikethrough = ann
                .get("strikethrough")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let underline = ann
                .get("underline")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            if code {
                text = format!("`{text}`");
            } else {
                if bold {
                    text = format!("**{text}**");
                }
                if italic {
                    text = format!("*{text}*");
                }
                if strikethrough {
                    text = format!("~~{text}~~");
                }
                if underline {
                    text = format!("<u>{text}</u>");
                }
            }
        }

        // Apply link
        if let Some(url) = href {
            text = format!("[{text}]({url})");
        }

        result.push_str(&text);
    }
    result
}

fn extract_caption(type_data: Option<&Value>) -> String {
    type_data
        .and_then(|d| d.get("caption"))
        .and_then(|v| v.as_array())
        .map(|arr| rich_text_to_markdown(arr))
        .unwrap_or_default()
}

fn extract_file_url(type_data: Option<&Value>) -> String {
    if let Some(data) = type_data {
        let file_type = data.get("type").and_then(|v| v.as_str()).unwrap_or("");
        if let Some(url) = data
            .get(file_type)
            .and_then(|f| f.get("url"))
            .and_then(|v| v.as_str())
        {
            return url.to_string();
        }
        // Fallback: direct url field
        if let Some(url) = data.get("url").and_then(|v| v.as_str()) {
            return url.to_string();
        }
    }
    String::new()
}
