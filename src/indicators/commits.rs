use std::time::{Duration, SystemTime, UNIX_EPOCH};

use git2::Repository;

use crate::LlmName;

fn llm_email_to_name(email: &str) -> Option<LlmName> {
    Some(LlmName(match email {
        "noreply@anthropic.com" => "Claude",
        "198982749+Copilot@users.noreply.github.com" => "Copilot",
        // i don't know why there's two Copilots
        "175728472+Copilot@users.noreply.github.com" => "Copilot",
        "199175422+chatgpt-codex-connector[bot]@users.noreply.github.com" => "Codex",
        "qwen-coder@alibabacloud.com" => "Qwen",
        "noreply@z.ai" => "GLM",
        "cursoragent@cursor.com" => "Cursor",
        "junie@jetbrains.com" => "Junie",
        "176961590+gemini-code-assist[bot]@users.noreply.github.com" => "Gemini",
        "161369871+google-labs-jules[bot]@users.noreply.github.com" => "Jules",
        "165735046+greptile-apps[bot]@users.noreply.github.com" => "Greptile",
        "github@tryaether.ai" => "Aether",
        "136622811+coderabbitai[bot]@users.noreply.github.com" => "CodeRabbit",
        "240665456+kilo-code-bot[bot]@users.noreply.github.com" => "Kilo",
        "96075541+graphite-app[bot]@users.noreply.github.com" => "Graphite",
        _ => return None,
    }))
}

#[derive(Debug, Default)]
pub struct CommitAuthorsData {
    pub commits_per_llm: Box<[(LlmName, u64)]>,
    pub total_commits: u64,

    pub commits_per_llm_in_past_month: Box<[(LlmName, u64)]>,
    pub total_commits_in_past_month: u64,
}

pub fn check_commit_authors(repo: &Repository) -> eyre::Result<CommitAuthorsData> {
    let mut commits_per_llm = Vec::<(LlmName, u64)>::new();
    let mut total_commits = 0_u64;

    let mut commits_per_llm_in_past_month = Vec::<(LlmName, u64)>::new();
    let mut total_commits_in_past_month = 0_u64;

    let mut revwalk = repo.revwalk()?;
    revwalk.push_head()?;

    let mut inc_llm_commits = |name: LlmName, is_past_month: bool| {
        if let Some((_, v)) = commits_per_llm.iter_mut().find(|(k, _)| *k == name) {
            *v += 1;
        } else {
            commits_per_llm.push((name, 1));
        }

        if is_past_month {
            if let Some((_, v)) = commits_per_llm_in_past_month
                .iter_mut()
                .find(|(k, _)| *k == name)
            {
                *v += 1;
            } else {
                commits_per_llm_in_past_month.push((name, 1));
            }
        }
    };

    let seconds_since_epoch_1_month_ago = (SystemTime::now() - Duration::from_hours(24 * 30))
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    for id in revwalk {
        let id = id.unwrap();
        let commit = repo.find_commit(id).unwrap();
        let is_past_month = (commit.time().seconds() as u64) > seconds_since_epoch_1_month_ago;

        total_commits += 1;
        if is_past_month {
            total_commits_in_past_month += 1;
        }

        if let Some(email) = commit.author().email()
            && let Some(llm_name) = llm_email_to_name(email)
        {
            inc_llm_commits(llm_name, is_past_month);
            // only count the first one
            continue;
        }
        if let Some(message) = commit.message() {
            for line in message.lines() {
                if line.to_lowercase().starts_with("co-authored-by: ") {
                    let Some(email) = line
                        .strip_suffix('>')
                        .and_then(|l| l.split('<').next_back())
                    else {
                        continue;
                    };
                    if let Some(llm_name) = llm_email_to_name(email) {
                        inc_llm_commits(llm_name, is_past_month);
                        // only count the first one
                        break;
                    }
                }
            }
        }

        total_commits += 1;
    }

    Ok(CommitAuthorsData {
        commits_per_llm: commits_per_llm.into(),
        total_commits,
        commits_per_llm_in_past_month: commits_per_llm_in_past_month.into(),
        total_commits_in_past_month,
    })
}
