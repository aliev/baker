use crate::{
    error::{Error, Result},
    loader::{LoadedTemplate, TemplateSourceInfo},
    prompt::confirm,
};
use std::fs;
use std::path::{Path, PathBuf};
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

        if repo_url.contains('@') && repo_url.contains(':') && !repo_url.contains("://") {
            if let Some(colon_pos) = repo_url.rfind(':') {
                let path_part = &repo_url[colon_pos + 1..];
                if !path_part.is_empty() {
                    let name = path_part
                        .split('/')
                        .next_back()
                        .unwrap_or("template")
                        .trim_end_matches(".git");
                    return if name.is_empty() {
                        "template".to_string()
                    } else {
                        name.to_string()
                    };
                }
            }
            return "template".to_string();
        }

        if let Ok(url) = Url::parse(repo_url) {
            if matches!(url.scheme(), "http" | "https" | "git" | "ssh" | "file") {
                if let Some(name) = url
                    .path_segments()
                    .and_then(|mut segments| segments.rfind(|s| !s.is_empty()))
                {
                    let name = name.trim_end_matches(".git");
                    if !name.is_empty() {
                        return name.to_string();
                    }
                }
            }
        }

        let path_name = Path::new(repo_url)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(repo_url);
        let path_name = path_name.trim_end_matches(".git");
        if !path_name.is_empty() && path_name != repo_url {
            return path_name.to_string();
        }

        let result =
            repo_url.rsplit(['/', '\\']).next().unwrap_or("").trim_end_matches(".git");

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
    ///
    /// SSH format is detected via heuristic: contains `@` and `:` but not `://`,
    /// with a valid-looking hostname (contains `.`) and a path containing `/`.
    pub fn is_git_url(s: &str) -> bool {
        if let Ok(url) = Url::parse(s) {
            return matches!(url.scheme(), "http" | "https" | "git" | "ssh");
        }

        if s.contains('@') && s.contains(':') && !s.contains("://") {
            if let Some(at_pos) = s.find('@') {
                if let Some(colon_pos) = s.rfind(':') {
                    if colon_pos > at_pos {
                        let user_part = &s[..at_pos];
                        let host_part = &s[at_pos + 1..colon_pos];
                        let path_part = &s[colon_pos + 1..];

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

    fn home_dir() -> Option<PathBuf> {
        std::env::var_os("HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("USERPROFILE").map(PathBuf::from))
            .or_else(|| {
                let home_drive = std::env::var_os("HOMEDRIVE")?;
                let home_path = std::env::var_os("HOMEPATH")?;
                let mut path = PathBuf::from(home_drive);
                path.push(home_path);
                Some(path)
            })
    }

    fn remote_callbacks() -> git2::RemoteCallbacks<'static> {
        let home_dir = Self::home_dir();

        let mut callbacks = git2::RemoteCallbacks::new();
        callbacks.credentials(move |_url, username_from_url, allowed_types| {
            if allowed_types.contains(git2::CredentialType::USERNAME) {
                return git2::Cred::username(username_from_url.unwrap_or("git"));
            }

            let username = username_from_url.unwrap_or("git");

            if allowed_types.contains(git2::CredentialType::SSH_KEY) {
                if let Ok(cred) = git2::Cred::ssh_key_from_agent(username) {
                    return Ok(cred);
                }

                if let Some(home_dir) = home_dir.as_ref() {
                    let key_path = home_dir.join(".ssh").join("id_rsa");
                    if key_path.exists() {
                        if let Ok(cred) =
                            git2::Cred::ssh_key(username, None, &key_path, None)
                        {
                            return Ok(cred);
                        }
                    }
                }
            }

            git2::Cred::default()
        });

        callbacks
    }

    /// Recursively initializes and updates all submodules in a repository.
    fn init_submodules(&self, repo: &git2::Repository) -> Result<()> {
        for mut submodule in repo.submodules()? {
            let submodule_name = submodule.name().unwrap_or("unknown").to_string();
            log::debug!("Initializing submodule: {}", submodule_name);
            submodule.init(false)?;

            let mut fetch_opts = git2::FetchOptions::new();
            fetch_opts.remote_callbacks(Self::remote_callbacks());
            let mut submodule_update_opts = git2::SubmoduleUpdateOptions::new();
            submodule_update_opts.fetch(fetch_opts);

            submodule.update(true, Some(&mut submodule_update_opts))?;

            if let Ok(sub_repo) = submodule.open() {
                self.init_submodules(&sub_repo)?;
            }
        }
        Ok(())
    }
}

impl<S: AsRef<str>> GitLoader<S> {
    fn default_clone_path(&self) -> Result<PathBuf> {
        Ok(std::env::current_dir()?.join(Self::extract_repo_name(self.repo.as_ref())))
    }

    pub(crate) fn load_into_parent(&self, parent: &Path) -> Result<LoadedTemplate> {
        self.load_into_path(parent.join(Self::extract_repo_name(self.repo.as_ref())))
    }

    fn load_into_path(&self, clone_path: PathBuf) -> Result<LoadedTemplate> {
        let repo_url = self.repo.as_ref();

        log::debug!("Cloning repository '{repo_url}'");

        if clone_path.exists() {
            let response = confirm(
                self.skip_overwrite_check,
                format!(
                    "Directory '{}' already exists. Replace it?",
                    clone_path.display()
                ),
            )?;
            if response {
                fs::remove_dir_all(&clone_path)?;
            } else {
                log::debug!("Using existing directory '{}'", clone_path.display());
                let source = read_git_source_info(repo_url, &clone_path)?;
                return Ok(LoadedTemplate { root: clone_path, source });
            }
        }

        log::debug!("Cloning to '{}'", clone_path.display());

        let mut fetch_opts = git2::FetchOptions::new();
        fetch_opts.remote_callbacks(Self::remote_callbacks());

        let mut builder = git2::build::RepoBuilder::new();
        builder.fetch_options(fetch_opts);

        match builder.clone(repo_url, &clone_path) {
            Ok(repo) => {
                self.init_submodules(&repo)?;
                let source = extract_source_info_from_repo(repo_url, &repo);
                Ok(LoadedTemplate { root: clone_path, source })
            }
            Err(e) => Err(Error::Git2Error(e)),
        }
    }
}

impl<S: AsRef<str>> TemplateLoader for GitLoader<S> {
    /// Loads a template by cloning a git repository.
    ///
    /// # Returns
    /// * `Result<LoadedTemplate>` - Loaded template with path and git source metadata
    fn load(&self) -> Result<LoadedTemplate> {
        self.load_into_path(self.default_clone_path()?)
    }
}

/// Extract `TemplateSourceInfo` from an already-opened `git2::Repository`.
fn extract_source_info_from_repo(
    url: &str,
    repo: &git2::Repository,
) -> TemplateSourceInfo {
    let (commit, tag) = match repo.head() {
        Ok(head) => {
            let commit =
                head.peel_to_commit().map(|c| c.id().to_string()).unwrap_or_default();
            let tag = find_tag_at_head(repo, &commit);
            (commit, tag)
        }
        Err(_) => (String::new(), None),
    };

    TemplateSourceInfo::Git { url: url.to_string(), commit, tag }
}

/// Open an existing repository and extract source info.
///
/// Falls back to minimal info (URL only) if the repository cannot be opened.
fn read_git_source_info(url: &str, path: &std::path::Path) -> Result<TemplateSourceInfo> {
    match git2::Repository::open(path) {
        Ok(repo) => Ok(extract_source_info_from_repo(url, &repo)),
        Err(_) => Ok(TemplateSourceInfo::Git {
            url: url.to_string(),
            commit: String::new(),
            tag: None,
        }),
    }
}

/// Find the first tag name pointing at the given commit SHA, if any.
/// Annotated tags are peeled to their underlying commit before comparison.
fn find_tag_at_head(repo: &git2::Repository, head_commit: &str) -> Option<String> {
    let tags = repo.tag_names(None).ok()?;
    for tag_name in tags.iter().flatten().flatten() {
        if let Ok(obj) = repo.revparse_single(&format!("refs/tags/{tag_name}")) {
            let commit_id = if obj.kind() == Some(git2::ObjectType::Tag) {
                obj.peel(git2::ObjectType::Commit).ok().map(|c| c.id().to_string())
            } else {
                Some(obj.id().to_string())
            };
            if commit_id.as_deref() == Some(head_commit) {
                return Some(tag_name.to_string());
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, path::Path};
    use tempfile::tempdir;

    fn init_git_repo(path: &Path) -> String {
        let repo = git2::Repository::init(path).expect("init repository");
        fs::write(path.join("README.md"), "hello").expect("write file");

        let mut index = repo.index().expect("open index");
        index.add_path(Path::new("README.md")).expect("add file to index");
        let tree_id = index.write_tree().expect("write tree");
        let tree = repo.find_tree(tree_id).expect("find tree");
        let sig = git2::Signature::now("tester", "tester@example.com")
            .expect("create signature");

        repo.commit(Some("HEAD"), &sig, &sig, "init", &tree, &[])
            .expect("create commit")
            .to_string()
    }

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

    #[test]
    fn test_extract_repo_name_local_paths() {
        assert_eq!(GitLoader::<String>::extract_repo_name("/tmp/demo_repo"), "demo_repo");
        assert_eq!(
            GitLoader::<String>::extract_repo_name("C:\\Users\\runner\\demo_repo"),
            "demo_repo"
        );
        assert_eq!(
            GitLoader::<String>::extract_repo_name("file:///tmp/demo_repo.git"),
            "demo_repo"
        );
    }

    #[test]
    fn test_extract_source_info_from_repo_and_tag_lookup() {
        let dir = tempdir().expect("create temp dir");
        let commit = init_git_repo(dir.path());
        let repo = git2::Repository::open(dir.path()).expect("open repository");
        let sig = git2::Signature::now("tester", "tester@example.com")
            .expect("create signature");
        let head_obj = repo.revparse_single("HEAD").expect("resolve HEAD");
        repo.tag("v1.0.0", &head_obj, &sig, "release", false).expect("create tag");

        let source = extract_source_info_from_repo("https://example.com/repo.git", &repo);
        match source {
            TemplateSourceInfo::Git { url, commit: found_commit, tag } => {
                assert_eq!(url, "https://example.com/repo.git");
                assert_eq!(found_commit, commit);
                assert_eq!(tag, Some("v1.0.0".to_string()));
            }
            _ => panic!("expected git source info"),
        }

        assert_eq!(find_tag_at_head(&repo, &commit), Some("v1.0.0".to_string()));
        assert_eq!(find_tag_at_head(&repo, "deadbeef"), None);
    }

    #[test]
    fn test_read_git_source_info_handles_repo_and_non_repo_paths() {
        let repo_dir = tempdir().expect("create repo dir");
        let commit = init_git_repo(repo_dir.path());

        let source =
            read_git_source_info("https://example.com/repo.git", repo_dir.path())
                .expect("read git source info");
        match source {
            TemplateSourceInfo::Git { url, commit: found_commit, tag } => {
                assert_eq!(url, "https://example.com/repo.git");
                assert_eq!(found_commit, commit);
                assert!(tag.is_none());
            }
            _ => panic!("expected git source info"),
        }

        let non_repo_dir = tempdir().expect("create non-repo dir");
        let fallback =
            read_git_source_info("https://example.com/repo.git", non_repo_dir.path())
                .expect("read fallback source info");
        match fallback {
            TemplateSourceInfo::Git { url, commit, tag } => {
                assert_eq!(url, "https://example.com/repo.git");
                assert!(commit.is_empty());
                assert!(tag.is_none());
            }
            _ => panic!("expected git fallback source info"),
        }
    }

    #[test]
    fn test_git_loader_loads_local_repo_and_replaces_existing_directory_when_skipped() {
        let source_parent = tempdir().expect("create source parent");
        let source_repo = source_parent.path().join("sample_repo");
        fs::create_dir_all(&source_repo).expect("create source repo dir");
        let commit = init_git_repo(&source_repo);

        let workspace = tempdir().expect("create workspace");
        let repo_name = GitLoader::<String>::extract_repo_name(
            source_repo.to_str().expect("source repo path"),
        );
        let existing_clone_path = workspace.path().join(&repo_name);
        fs::create_dir_all(&existing_clone_path).expect("create pre-existing dir");
        fs::write(existing_clone_path.join("old.txt"), "old").expect("write old content");

        let loader = GitLoader::new(
            source_repo.to_str().expect("source repo path").to_string(),
            true,
        );
        let loaded =
            loader.load_into_parent(workspace.path()).expect("load local repository");

        assert_eq!(loaded.root, existing_clone_path);
        assert!(
            git2::Repository::open(&loaded.root).is_ok(),
            "cloned repo should be openable as a git repository"
        );
        assert!(!loaded.root.join("old.txt").exists(), "old dir should be replaced");

        match loaded.source {
            TemplateSourceInfo::Git { url, commit: found_commit, tag } => {
                assert_eq!(url, source_repo.to_string_lossy().to_string());
                assert_eq!(found_commit, commit);
                assert!(tag.is_none());
            }
            _ => panic!("expected git source info"),
        }
    }

    #[test]
    fn test_load_into_parent_uses_repo_name_for_target_path() {
        let source_parent = tempdir().expect("create source parent");
        let source_repo = source_parent.path().join("demo_repo");
        fs::create_dir_all(&source_repo).expect("create source repo dir");
        init_git_repo(&source_repo);

        let workspace = tempdir().expect("create workspace");
        let loader = GitLoader::new(
            source_repo.to_str().expect("source repo path").to_string(),
            true,
        );

        let loaded = loader
            .load_into_parent(workspace.path())
            .expect("load local repository into parent");

        assert_eq!(loaded.root, workspace.path().join("demo_repo"));
    }
}
