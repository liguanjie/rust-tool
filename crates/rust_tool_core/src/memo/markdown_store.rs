use super::store::{current_timestamp, MemoMetadata};
use serde::{Deserialize, Serialize};
use std::collections::{hash_map::DefaultHasher, HashMap};
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

#[derive(Debug, Clone)]
pub struct MarkdownDocument {
    pub metadata: MemoMetadata,
    pub markdown: String,
}

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct MarkdownFrontmatter {
    id: Option<String>,
    title: Option<String>,
    summary: Option<String>,
    updated_at: Option<i64>,
}

pub fn list_documents(
    root: &Path,
    db_metadata_by_file: &HashMap<String, MemoMetadata>,
) -> Result<Vec<MemoMetadata>, String> {
    let mut documents = Vec::new();
    if !root.exists() {
        return Ok(documents);
    }

    for path in walk_markdown_files(root)? {
        let file_name = relative_file_name(root, &path)?;
        let content = fs::read_to_string(&path)
            .map_err(|error| format!("Failed to read Markdown document: {error:?}"))?;
        let db_meta = db_metadata_by_file.get(&file_name);
        documents.push(metadata_from_content(&file_name, &path, &content, db_meta));
    }

    documents.sort_by(|left, right| right.updated_at.cmp(&left.updated_at));
    Ok(documents)
}

pub fn read_document(
    root: &Path,
    file_name: &str,
    db_meta: Option<&MemoMetadata>,
) -> Result<MarkdownDocument, String> {
    let path = root.join(file_name);
    let content = fs::read_to_string(&path)
        .map_err(|error| format!("Failed to read Markdown document: {error:?}"))?;
    let metadata = metadata_from_content(file_name, &path, &content, db_meta);
    let markdown = strip_frontmatter(&content)
        .trim_start_matches('\n')
        .to_string();

    Ok(MarkdownDocument { metadata, markdown })
}

pub fn render_document(metadata: &MemoMetadata, markdown: &str) -> Result<String, String> {
    let body = strip_frontmatter(markdown).trim_start_matches('\n');
    let frontmatter = MarkdownFrontmatter {
        id: Some(metadata.id.clone()),
        title: Some(metadata.title.clone()),
        summary: Some(metadata.summary.clone()),
        updated_at: Some(metadata.updated_at),
    };
    let yaml = serde_yaml::to_string(&frontmatter)
        .map_err(|error| format!("Failed to serialize Markdown frontmatter: {error:?}"))?;
    let yaml = yaml.trim_start_matches("---\n").trim_end();

    Ok(format!("---\n{yaml}\n---\n\n{body}"))
}

pub fn strip_frontmatter(content: &str) -> &str {
    let Some((_, body)) = split_frontmatter(content) else {
        return content;
    };
    body
}

fn walk_markdown_files(root: &Path) -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();
    if !root.is_dir() {
        return Ok(files);
    }

    for entry in fs::read_dir(root).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let path = entry.path();
        if path.is_dir() {
            files.extend(walk_markdown_files(&path)?);
        } else if path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("md"))
        {
            files.push(path);
        }
    }

    Ok(files)
}

fn metadata_from_content(
    file_name: &str,
    path: &Path,
    content: &str,
    db_meta: Option<&MemoMetadata>,
) -> MemoMetadata {
    let (frontmatter, markdown) = match split_frontmatter(content) {
        Some((frontmatter, markdown)) => (parse_frontmatter(frontmatter), markdown),
        None => (MarkdownFrontmatter::default(), content),
    };

    MemoMetadata {
        id: first_non_empty(
            frontmatter.id.as_deref(),
            db_meta.map(|meta| meta.id.as_str()),
        )
        .map(str::to_string)
        .unwrap_or_else(|| stable_file_id(file_name)),
        file_name: file_name.to_string(),
        title: first_non_empty(
            frontmatter.title.as_deref(),
            db_meta.map(|meta| meta.title.as_str()),
        )
        .map(str::to_string)
        .unwrap_or_else(|| derive_title(file_name, markdown)),
        summary: first_non_empty(
            frontmatter.summary.as_deref(),
            db_meta.map(|meta| meta.summary.as_str()),
        )
        .map(str::to_string)
        .unwrap_or_else(|| derive_summary(markdown)),
        updated_at: frontmatter
            .updated_at
            .or_else(|| db_meta.map(|meta| meta.updated_at))
            .unwrap_or_else(|| file_updated_at(path)),
    }
}

fn parse_frontmatter(frontmatter: &str) -> MarkdownFrontmatter {
    serde_yaml::from_str(frontmatter).unwrap_or_default()
}

fn split_frontmatter(content: &str) -> Option<(&str, &str)> {
    let mut offset = 0;
    let mut lines = content.split_inclusive('\n');
    let first_line = lines.next()?;
    if first_line.trim_end_matches(['\r', '\n']) != "---" {
        return None;
    }

    offset += first_line.len();
    let frontmatter_start = offset;
    let mut frontmatter_end = frontmatter_start;

    for line in lines {
        let line_start = offset;
        offset += line.len();
        if line.trim_end_matches(['\r', '\n']) == "---" {
            return Some((
                &content[frontmatter_start..frontmatter_end],
                &content[offset..],
            ));
        }
        frontmatter_end = line_start + line.len();
    }

    None
}

fn relative_file_name(root: &Path, path: &Path) -> Result<String, String> {
    let relative = path
        .strip_prefix(root)
        .map_err(|error| format!("Failed to resolve Markdown path: {error:?}"))?;
    Ok(relative.to_string_lossy().replace('\\', "/"))
}

fn first_non_empty<'a>(first: Option<&'a str>, second: Option<&'a str>) -> Option<&'a str> {
    first
        .filter(|value| !value.trim().is_empty())
        .or_else(|| second.filter(|value| !value.trim().is_empty()))
}

fn derive_title(file_name: &str, markdown: &str) -> String {
    markdown
        .lines()
        .find_map(|line| line.trim().strip_prefix("# "))
        .map(str::trim)
        .filter(|title| !title.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| {
            Path::new(file_name)
                .file_stem()
                .and_then(|stem| stem.to_str())
                .unwrap_or("Untitled")
                .to_string()
        })
}

fn derive_summary(markdown: &str) -> String {
    let summary = markdown
        .lines()
        .map(|line| line.trim().trim_start_matches('#').trim())
        .find(|line| !line.is_empty())
        .unwrap_or("Markdown 文档");
    summary.chars().take(80).collect()
}

fn stable_file_id(file_name: &str) -> String {
    let mut hasher = DefaultHasher::new();
    file_name.hash(&mut hasher);
    format!("file-{:016x}", hasher.finish())
}

fn file_updated_at(path: &Path) -> i64 {
    fs::metadata(path)
        .and_then(|metadata| metadata.modified())
        .ok()
        .and_then(|modified| modified.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or_else(current_timestamp)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn make_test_dir(name: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("rusttool_markdown_{}_{}", name, stamp));
        fs::create_dir_all(&dir).expect("test temp dir");
        dir
    }

    #[test]
    fn lists_markdown_files_without_database_metadata() {
        let temp_dir = make_test_dir("list");
        let nested = temp_dir.join("servers");
        fs::create_dir_all(&nested).expect("nested dir");
        fs::write(
            nested.join("prod.md"),
            "---\nid: prod-server\ntitle: 生产服务器\nsummary: SSH 登录信息\nupdatedAt: 42\n---\n\n# 生产服务器",
        )
        .expect("write doc");

        let docs = list_documents(&temp_dir, &HashMap::new()).expect("list docs");

        assert_eq!(docs.len(), 1);
        assert_eq!(docs[0].id, "prod-server");
        assert_eq!(docs[0].file_name, "servers/prod.md");
        assert_eq!(docs[0].title, "生产服务器");
        assert_eq!(docs[0].summary, "SSH 登录信息");
        assert_eq!(docs[0].updated_at, 42);

        let _ = fs::remove_dir_all(temp_dir);
    }

    #[test]
    fn renders_and_strips_frontmatter() {
        let meta = MemoMetadata {
            id: "doc-1".to_string(),
            file_name: "doc.md".to_string(),
            title: "文档".to_string(),
            summary: "摘要".to_string(),
            updated_at: 7,
        };
        let rendered = render_document(&meta, "# 文档\n\n正文").expect("render doc");

        assert!(rendered.starts_with("---\n"));
        assert!(rendered.contains("id: doc-1"));
        assert_eq!(strip_frontmatter(&rendered).trim(), "# 文档\n\n正文");
    }
}
