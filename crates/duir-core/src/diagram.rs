/// Supported diagram languages.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiagramLang {
    Mermaid,
    PlantUml,
    Graphviz,
}

/// A detected diagram block within a note.
#[derive(Debug, Clone)]
pub struct DiagramBlock {
    pub lang: DiagramLang,
    pub source: String,
}

/// Known diagram language tags.
const DIAGRAM_TAGS: &[(&str, DiagramLang)] = &[
    ("mermaid", DiagramLang::Mermaid),
    ("plantuml", DiagramLang::PlantUml),
    ("dot", DiagramLang::Graphviz),
    ("graphviz", DiagramLang::Graphviz),
];

/// Extract all diagram blocks from markdown content.
#[must_use]
pub fn extract_diagrams(content: &str) -> Vec<DiagramBlock> {
    let mut diagrams = Vec::new();
    let mut lines = content.lines();

    while let Some(line) = lines.next() {
        if let Some(lang) = parse_fence_lang(line) {
            let mut source = String::new();
            for inner in lines.by_ref() {
                if inner.starts_with("```") {
                    break;
                }
                if !source.is_empty() {
                    source.push('\n');
                }
                source.push_str(inner);
            }
            if !source.is_empty() {
                diagrams.push(DiagramBlock { lang, source });
            }
        }
    }

    diagrams
}

fn parse_fence_lang(line: &str) -> Option<DiagramLang> {
    let tag = line.strip_prefix("```")?.trim();
    DIAGRAM_TAGS
        .iter()
        .find(|(name, _)| tag.eq_ignore_ascii_case(name))
        .map(|(_, lang)| lang.clone())
}

/// Render a diagram to PNG bytes using an external tool.
///
/// # Errors
/// Returns an error if the tool is not found or rendering fails.
pub fn render_diagram(block: &DiagramBlock, tool_paths: &ToolPaths) -> crate::Result<Vec<u8>> {
    let (cmd, args, input_ext) = match block.lang {
        DiagramLang::Mermaid => (
            &tool_paths.mmdc,
            vec!["-i", "{input}", "-o", "{output}", "-b", "transparent"],
            "mmd",
        ),
        DiagramLang::PlantUml => (&tool_paths.plantuml, vec!["-tpng", "-pipe"], "puml"),
        DiagramLang::Graphviz => (&tool_paths.dot, vec!["-Tpng", "-o", "{output}", "{input}"], "dot"),
    };

    // PlantUML uses stdin/stdout pipe
    if block.lang == DiagramLang::PlantUml {
        return render_via_pipe(cmd, &args, &block.source);
    }

    // Mermaid and Graphviz use temp files
    render_via_files(cmd, &args, &block.source, input_ext)
}

/// Tool paths configuration.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct ToolPaths {
    pub mmdc: String,
    pub plantuml: String,
    pub dot: String,
}

impl Default for ToolPaths {
    fn default() -> Self {
        Self {
            mmdc: "mmdc".to_owned(),
            plantuml: "plantuml".to_owned(),
            dot: "dot".to_owned(),
        }
    }
}

fn render_via_pipe(cmd: &str, args: &[&str], source: &str) -> crate::Result<Vec<u8>> {
    use std::io::Write;
    use std::process::{Command, Stdio};

    let mut child = Command::new(cmd)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| crate::OmelaError::Other(format!("Failed to run {cmd}: {e}")))?;

    if let Some(stdin) = child.stdin.as_mut() {
        stdin
            .write_all(source.as_bytes())
            .map_err(|e| crate::OmelaError::Other(format!("Write to {cmd}: {e}")))?;
    }
    drop(child.stdin.take());

    let output = child
        .wait_with_output()
        .map_err(|e| crate::OmelaError::Other(format!("{cmd} failed: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(crate::OmelaError::Other(format!("{cmd} error: {stderr}")));
    }

    Ok(output.stdout)
}

fn render_via_files(cmd: &str, args_template: &[&str], source: &str, input_ext: &str) -> crate::Result<Vec<u8>> {
    let dir = std::env::temp_dir().join("duir-diagrams");
    std::fs::create_dir_all(&dir).map_err(|e| crate::OmelaError::Other(format!("Temp dir: {e}")))?;

    let id = std::process::id();
    let input_path = dir.join(format!("diagram-{id}.{input_ext}"));
    let output_path = dir.join(format!("diagram-{id}.png"));

    std::fs::write(&input_path, source).map_err(|e| crate::OmelaError::Other(format!("Write input: {e}")))?;

    let args: Vec<String> = args_template
        .iter()
        .map(|a| {
            a.replace("{input}", &input_path.to_string_lossy())
                .replace("{output}", &output_path.to_string_lossy())
        })
        .collect();

    let output = std::process::Command::new(cmd)
        .args(&args)
        .output()
        .map_err(|e| crate::OmelaError::Other(format!("Failed to run {cmd}: {e}")))?;

    // Clean up input
    std::fs::remove_file(&input_path).ok();

    if !output.status.success() {
        std::fs::remove_file(&output_path).ok();
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(crate::OmelaError::Other(format!("{cmd} error: {stderr}")));
    }

    let png = std::fs::read(&output_path).map_err(|e| crate::OmelaError::Other(format!("Read output: {e}")))?;

    std::fs::remove_file(&output_path).ok();

    Ok(png)
}

#[cfg(test)]
#[allow(clippy::indexing_slicing)]
mod tests {
    use super::*;

    #[test]
    fn detect_mermaid() {
        let content = "# Title\n```mermaid\ngraph TD\nA-->B\n```\nsome text";
        let blocks = extract_diagrams(content);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].lang, DiagramLang::Mermaid);
        assert_eq!(blocks[0].source, "graph TD\nA-->B");
    }

    #[test]
    fn detect_plantuml() {
        let content = "```plantuml\n@startuml\nAlice -> Bob\n@enduml\n```";
        let blocks = extract_diagrams(content);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].lang, DiagramLang::PlantUml);
    }

    #[test]
    fn detect_dot() {
        let content = "```dot\ndigraph { A -> B }\n```";
        let blocks = extract_diagrams(content);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].lang, DiagramLang::Graphviz);
    }

    #[test]
    fn detect_graphviz_alias() {
        let content = "```graphviz\ndigraph { A -> B }\n```";
        let blocks = extract_diagrams(content);
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].lang, DiagramLang::Graphviz);
    }

    #[test]
    fn detect_multiple() {
        let content = "```mermaid\nA\n```\ntext\n```dot\nB\n```";
        let blocks = extract_diagrams(content);
        assert_eq!(blocks.len(), 2);
    }

    #[test]
    fn ignore_non_diagram_fences() {
        let content = "```rust\nfn main() {}\n```";
        let blocks = extract_diagrams(content);
        assert!(blocks.is_empty());
    }

    #[test]
    fn empty_block_ignored() {
        let content = "```mermaid\n```";
        let blocks = extract_diagrams(content);
        assert!(blocks.is_empty());
    }

    #[test]
    fn case_insensitive() {
        let content = "```Mermaid\ngraph TD\n```";
        let blocks = extract_diagrams(content);
        assert_eq!(blocks.len(), 1);
    }
}
