use std::{collections::HashSet, fs, path::Path};

pub const LLM_PATHS: &[&str] = &[
    "CLAUDE.md",
    "AGENTS.md",
    "SKILL.md",
    ".mcp.json",
    ".windsurf",
    ".kilocode",
    ".kiro",
    ".claude",
    ".codex",
    ".aider.conf.yml",
    ".clinerules",
    ".cursor",
    ".roo",
    ".junie",
    ".gemini",
];

#[derive(Default, Debug)]
pub struct LlmFiles {
    pub in_worktree: Box<[&'static str]>,
    pub in_gitignore: Box<[&'static str]>,
}

pub fn check_for_llm_files(path: &Path) -> eyre::Result<LlmFiles> {
    let mut in_worktree = HashSet::<&'static str>::new();
    let mut in_gitignore = HashSet::<&'static str>::new();

    for entry in path.read_dir()? {
        let entry = entry?;
        let file_name = entry.file_name();
        let Some(file_name) = file_name.to_str() else {
            continue;
        };

        if file_name == ".gitignore" {
            let gitignore_contents = fs::read_to_string(entry.path())?;
            for mut line in gitignore_contents.lines() {
                // doesn't need to parse perfectly, just well enough
                if let Some((line_without_comment, _)) = line.split_once('#') {
                    line = line_without_comment;
                }
                line = line.trim().trim_matches('/');
                let Some(line) = line.split('/').next_back() else {
                    continue;
                };

                let Some(llm_file) = LLM_PATHS.iter().find(|f| f.eq_ignore_ascii_case(line)) else {
                    continue;
                };
                in_gitignore.insert(llm_file);
            }
            continue;
        }

        let Some(llm_file) = LLM_PATHS.iter().find(|f| f.eq_ignore_ascii_case(file_name)) else {
            continue;
        };
        in_worktree.insert(llm_file);
    }

    Ok(LlmFiles {
        in_worktree: in_worktree.into_iter().collect(),
        in_gitignore: in_gitignore.into_iter().collect(),
    })
}
