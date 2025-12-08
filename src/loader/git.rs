use crate::{
    error::{Error, Result},
    prompt::confirm,
};
use std::fs;
use std::path::PathBuf;
use url::Url;

use crate::loader::interface::TemplateLoader;

/// Loader for templates from git repositories.
pub struct GitLoader<S: AsRef<str>> {
    repo: S,
    skip_overwrite_check: bool,
}

impl<S: AsRef<str>> GitLoader<S> {
    /// Creates a new GitLoader instance.
    pub fn new(repo: S, skip_overwrite_check: bool) -> Self {
        Self { repo, skip_overwrite_check }
    }

    /// Extracts repository name from various git URL formats.
    ///
    /// Supports:
    /// - HTTPS: https://github.com/user/repo.git -> repo
    /// - SSH: git@github.com:user/repo.git -> repo
    /// - SSH without .git: git@github.com:user/repo -> repo
    pub fn extract_repo_name(repo_url: &str) -> String {
        if repo_url.is_empty() {
            return "template".to_string();
        }

        // Handle SSH format: git@host:user/repo or user@host:user/repo
        if repo_url.contains('@') && repo_url.contains(':') && !repo_url.contains("://") {
            if let Some(colon_pos) = repo_url.rfind(':') {
                let path_part = &repo_url[colon_pos + 1..];
                if !path_part.is_empty() {
                    return path_part
                        .split('/')
                        .next_back()
                        .unwrap_or("template")
                        .trim_end_matches(".git")
                        .to_string();
                }
            }
        }

        // Handle standard URLs (HTTPS, git://, etc.)
        let result =
            repo_url.split('/').next_back().unwrap_or("").trim_end_matches(".git");

        if result.is_empty() || result.contains('@') || result.contains(':') {
            "template".to_string()
        } else {
            result.to_string()
        }
    }

    /// Determines if a string represents a git repository URL.
    ///
    /// Supports:
    /// - HTTPS URLs: https://github.com/user/repo
    /// - Git URLs: git://github.com/user/repo
    /// - SSH URLs: git@github.com:user/repo
    /// - SSH URLs with explicit protocol: ssh://git@github.com/user/repo
    pub fn is_git_url(s: &str) -> bool {
        // Try to parse as standard URL first
        if let Ok(url) = Url::parse(s) {
            return matches!(url.scheme(), "http" | "https" | "git" | "ssh");
        }

        // Check for SSH format: git@host:path or user@host:path
        if s.contains('@') && s.contains(':') && !s.contains("://") {
            // Simple heuristic: if it contains @ and : but not ://, it's likely SSH format
            // Also check that the part after @ and before : looks like a hostname
            if let Some(at_pos) = s.find('@') {
                if let Some(colon_pos) = s.rfind(':') {
                    if colon_pos > at_pos {
                        let user_part = &s[..at_pos];
                        let host_part = &s[at_pos + 1..colon_pos];
                        let path_part = &s[colon_pos + 1..];

                        // More strict validation:
                        // - user part should look like a username (git, or valid username)
                        // - host should look like a hostname (contains . or known git hosts)
                        // - path should look like a repository path (contains /)
                        return !user_part.is_empty()
                            && !host_part.is_empty()
                            && !path_part.is_empty()
                            && (host_part.contains('.')
                                || host_part == "github.com"
                                || host_part == "gitlab.com"
                                || host_part == "bitbucket.org")
                            && path_part.contains('/');
                    }
                }
            }
        }

        false
    }
}
impl<S: AsRef<str>> TemplateLoader for GitLoader<S> {
    /// Loads a template by cloning a git repository.
    ///
    /// # Returns
    /// * `Result<PathBuf>` - Path to the cloned repository
    fn load(&self) -> Result<PathBuf> {
        let repo_url = self.repo.as_ref();

        log::debug!("Cloning repository '{repo_url}'");

        let repo_name = Self::extract_repo_name(repo_url);
        let clone_path = PathBuf::from(&repo_name);

        if clone_path.exists() {
            let response = confirm(
                self.skip_overwrite_check,
                format!("Directory '{repo_name}' already exists. Replace it?"),
            )?;
            if response {
                fs::remove_dir_all(&clone_path)?;
            } else {
                log::debug!("Using existing directory '{}'", clone_path.display());
                return Ok(clone_path);
            }
        }

        log::debug!("Cloning to '{}'", clone_path.display());
        let home = std::env::var("HOME").map_err(|e| {
            Error::Other(anyhow::anyhow!("Failed to get HOME directory: {}", e))
        })?;

        // Set up authentication callbacks
        let mut callbacks = git2::RemoteCallbacks::new();
        callbacks.credentials(|_url, username_from_url, _allowed_types| {
            git2::Cred::ssh_key(
                username_from_url.unwrap_or("git"),
                None,
                std::path::Path::new(&format!("{home}/.ssh/id_rsa")),
                None,
            )
        });

        // Configure fetch options with callbacks
        let mut fetch_opts = git2::FetchOptions::new();
        fetch_opts.remote_callbacks(callbacks);

        // Set up and perform clone
        let mut builder = git2::build::RepoBuilder::new();
        builder.fetch_options(fetch_opts);

        match builder.clone(repo_url, &clone_path) {
            Ok(_) => Ok(clone_path),
            Err(e) => Err(Error::Git2Error(e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_git_url_http() {
        assert!(GitLoader::<&str>::is_git_url("http://localhost:3000/user/repo"));
        assert!(GitLoader::<&str>::is_git_url("http://localhost:3000/user/repo.git"));
        assert!(GitLoader::<&str>::is_git_url("http://192.168.1.1/user/repo"));
        assert!(GitLoader::<&str>::is_git_url("http://gitea.local/user/repo.git"));
    }

    #[test]
    fn test_is_git_url_https() {
        assert!(GitLoader::<&str>::is_git_url("https://github.com/user/repo"));
        assert!(GitLoader::<&str>::is_git_url("https://github.com/user/repo.git"));
        assert!(GitLoader::<&str>::is_git_url("https://gitlab.com/user/repo"));
    }

    #[test]
    fn test_is_git_url_ssh() {
        assert!(GitLoader::<&str>::is_git_url("git@github.com:user/repo"));
        assert!(GitLoader::<&str>::is_git_url("git@github.com:user/repo.git"));
        assert!(GitLoader::<&str>::is_git_url("git@gitlab.com:user/repo"));
        assert!(GitLoader::<&str>::is_git_url("user@bitbucket.org:user/repo"));
    }

    #[test]
    fn test_is_git_url_git_protocol() {
        assert!(GitLoader::<&str>::is_git_url("git://github.com/user/repo"));
        assert!(GitLoader::<&str>::is_git_url("ssh://git@github.com/user/repo"));
    }

    #[test]
    fn test_is_git_url_local_paths() {
        assert!(!GitLoader::<&str>::is_git_url("/path/to/local/template"));
        assert!(!GitLoader::<&str>::is_git_url("./relative/path"));
        assert!(!GitLoader::<&str>::is_git_url("../parent/path"));
        assert!(!GitLoader::<&str>::is_git_url("template"));
        assert!(!GitLoader::<&str>::is_git_url("C:\\Windows\\Path"));
    }

    #[test]
    fn test_is_git_url_invalid_ssh() {
        // Should not match SSH-like strings that aren't actually git URLs
        assert!(!GitLoader::<&str>::is_git_url("user@localhost:file.txt"));
        assert!(!GitLoader::<&str>::is_git_url("name@email.com:something"));
        assert!(!GitLoader::<&str>::is_git_url("user@host"));
        assert!(!GitLoader::<&str>::is_git_url("@host:path"));
    }

    #[test]
    fn test_extract_repo_name_https() {
        assert_eq!(
            GitLoader::<String>::extract_repo_name("https://github.com/user/repo"),
            "repo"
        );
        assert_eq!(
            GitLoader::<String>::extract_repo_name("https://github.com/user/repo.git"),
            "repo"
        );
        assert_eq!(
            GitLoader::<String>::extract_repo_name(
                "https://gitlab.com/group/subgroup/repo.git"
            ),
            "repo"
        );
    }

    #[test]
    fn test_extract_repo_name_ssh() {
        assert_eq!(
            GitLoader::<String>::extract_repo_name("git@github.com:user/repo"),
            "repo"
        );
        assert_eq!(
            GitLoader::<String>::extract_repo_name("git@github.com:user/repo.git"),
            "repo"
        );
        assert_eq!(
            GitLoader::<String>::extract_repo_name("user@gitlab.com:group/repo"),
            "repo"
        );
    }

    #[test]
    fn test_extract_repo_name_edge_cases() {
        assert_eq!(GitLoader::<String>::extract_repo_name("invalid-url"), "invalid-url");
        assert_eq!(GitLoader::<String>::extract_repo_name(""), "template");
        assert_eq!(GitLoader::<String>::extract_repo_name("git@host:"), "template");
    }
}
