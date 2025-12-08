//! Integration tests for Git repository cloning using Gitea testcontainer.
//!
//! These tests verify that the GitLoader can successfully clone repositories
//! from a real Git server (Gitea running in a container).
//!
//! All tests share a single Gitea instance for efficiency.
//!
//! Run these tests with: `cargo test --test gitea_integration_tests -- --ignored`

use baker::cli::SkipConfirm::All;
use baker::cli::{run, Args};
use git2::{Cred, PushOptions, RemoteCallbacks, Repository, Signature};
use reqwest::blocking::Client;
use serde_json::json;
use std::fs;
use std::path::Path;
use std::sync::OnceLock;
use tempfile::TempDir;
use testcontainers::{
    core::{ExecCommand, IntoContainerPort, WaitFor},
    runners::SyncRunner,
    GenericImage, ImageExt,
};

/// Gitea container configuration
const GITEA_IMAGE: &str = "gitea/gitea";
const GITEA_TAG: &str = "1.25-rootless";
const GITEA_HTTP_PORT: u16 = 3000;

/// Test user credentials for Gitea
const TEST_USER: &str = "testuser";
const TEST_PASSWORD: &str = "Password123!";
const TEST_EMAIL: &str = "test@example.com";

/// Shared Gitea instance for all tests
static GITEA_INSTANCE: OnceLock<SharedGiteaEnv> = OnceLock::new();

/// Shared Gitea environment that persists across all tests
struct SharedGiteaEnv {
    #[allow(dead_code)]
    container: testcontainers::core::Container<GenericImage>,
    base_url: String,
    client: Client,
}

// Safety: The container and client are thread-safe for our read-only access patterns
unsafe impl Sync for SharedGiteaEnv {}
unsafe impl Send for SharedGiteaEnv {}

impl SharedGiteaEnv {
    fn new() -> Result<Self, Box<dyn std::error::Error>> {
        eprintln!("Starting shared Gitea container...");

        let container = GenericImage::new(GITEA_IMAGE, GITEA_TAG)
            .with_exposed_port(GITEA_HTTP_PORT.tcp())
            .with_wait_for(WaitFor::message_on_stdout("Starting new Web server"))
            .with_env_var("GITEA__security__INSTALL_LOCK", "true")
            .with_env_var("GITEA__database__DB_TYPE", "sqlite3")
            .with_env_var("GITEA__database__PATH", "/tmp/data/gitea.db")
            .with_env_var("GITEA__server__HTTP_PORT", GITEA_HTTP_PORT.to_string())
            .with_env_var("GITEA__server__DOMAIN", "localhost")
            .with_env_var("GITEA__server__ROOT_URL", "http://localhost:3000/")
            .with_env_var("GITEA__server__OFFLINE_MODE", "true")
            .with_env_var("GITEA__repository__DEFAULT_BRANCH", "main")
            .with_env_var("GITEA__service__DISABLE_REGISTRATION", "true")
            .with_env_var("GITEA__service__REQUIRE_SIGNIN_VIEW", "false")
            .with_env_var("GITEA__log__LEVEL", "Info")
            .start()?;

        eprintln!("Gitea container started successfully");

        let host_port = container
            .get_host_port_ipv4(GITEA_HTTP_PORT.tcp())
            .expect("Failed to get Gitea port");
        let base_url = format!("http://127.0.0.1:{}", host_port);

        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        // Wait for Gitea to be ready
        if !wait_for_gitea_ready(&base_url, &client, 60) {
            return Err("Gitea failed to become ready".into());
        }

        // Create admin user via container exec
        create_admin_user(&container)?;

        // Small delay to ensure user is fully persisted
        std::thread::sleep(std::time::Duration::from_secs(1));

        eprintln!("Shared Gitea environment ready at {}", base_url);

        Ok(Self { container, base_url, client })
    }

    fn create_repo(&self, repo_name: &str) -> Result<String, Box<dyn std::error::Error>> {
        create_gitea_repo(&self.base_url, &self.client, repo_name)
    }

    fn clone_url_with_auth(&self, repo_name: &str) -> String {
        let host_port = self.base_url.strip_prefix("http://127.0.0.1:").unwrap_or("3000");
        format!(
            "http://{}:{}@127.0.0.1:{}/{}/{}.git",
            TEST_USER, TEST_PASSWORD, host_port, TEST_USER, repo_name
        )
    }
}

/// Gets or initializes the shared Gitea instance
fn get_shared_gitea() -> &'static SharedGiteaEnv {
    GITEA_INSTANCE.get_or_init(|| {
        SharedGiteaEnv::new().expect("Failed to create shared Gitea environment")
    })
}

/// Creates a sample baker template in the given directory
fn create_sample_template(dir: &Path) {
    let baker_yaml = r#"schemaVersion: v1

questions:
  project_name:
    type: str
    help: Enter project name
    default: "my project"

  version:
    type: str
    help: Enter version
    default: "0.1.0"
"#;
    fs::write(dir.join("baker.yaml"), baker_yaml).expect("Failed to write baker.yaml");

    let template_content = r#"# {{ project_name }}

Version: {{ version }}

This is a sample project.
"#;
    fs::write(dir.join("README.md.baker.j2"), template_content)
        .expect("Failed to write template file");

    let src_dir = dir.join("src");
    fs::create_dir_all(&src_dir).expect("Failed to create src directory");
    fs::write(src_dir.join("main.txt"), "Hello from {{ project_name }}!")
        .expect("Failed to write main.txt");
}

/// Initializes a git repository and pushes to remote using libgit2
fn init_and_push_repo(
    local_path: &Path,
    remote_url: &str,
    username: &str,
    password: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("Initializing git repo at {:?}", local_path);

    let repo = Repository::init(local_path)?;
    let signature = Signature::now(username, TEST_EMAIL)?;

    let mut index = repo.index()?;
    index.add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)?;
    index.write()?;

    let tree_id = index.write_tree()?;
    let tree = repo.find_tree(tree_id)?;

    repo.commit(Some("HEAD"), &signature, &signature, "Initial commit", &tree, &[])?;

    repo.branch("main", &repo.head()?.peel_to_commit()?, true)?;
    repo.set_head("refs/heads/main")?;

    eprintln!("Initial commit created");

    let url_with_creds =
        remote_url.replace("://", &format!("://{}:{}@", username, password));
    eprintln!("Adding remote: {}", remote_url);

    repo.remote("origin", &url_with_creds)?;

    let mut callbacks = RemoteCallbacks::new();
    let user = username.to_string();
    let pass = password.to_string();
    callbacks.credentials(move |_url, _username_from_url, _allowed_types| {
        Cred::userpass_plaintext(&user, &pass)
    });

    eprintln!("Pushing to remote...");
    let mut remote = repo.find_remote("origin")?;
    let mut push_options = PushOptions::new();
    push_options.remote_callbacks(callbacks);

    remote.push(&["refs/heads/main:refs/heads/main"], Some(&mut push_options))?;

    eprintln!("Successfully pushed to remote");
    Ok(())
}

/// Creates a git branch with new content and pushes it
fn create_and_push_branch(
    repo: &Repository,
    branch_name: &str,
    baker_yaml_content: &str,
    username: &str,
    password: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let signature = Signature::now(username, TEST_EMAIL)?;

    let head_commit = repo.head()?.peel_to_commit()?;
    let branch = repo.branch(branch_name, &head_commit, false)?;
    repo.set_head(branch.get().name().unwrap())?;
    repo.checkout_head(Some(git2::build::CheckoutBuilder::default().force()))?;

    let workdir = repo.workdir().ok_or("No workdir")?;
    fs::write(workdir.join("baker.yaml"), baker_yaml_content)?;

    let mut index = repo.index()?;
    index.add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)?;
    index.write()?;

    let tree_id = index.write_tree()?;
    let tree = repo.find_tree(tree_id)?;

    repo.commit(
        Some("HEAD"),
        &signature,
        &signature,
        &format!("{} branch changes", branch_name),
        &tree,
        &[&head_commit],
    )?;

    let mut callbacks = RemoteCallbacks::new();
    let user = username.to_string();
    let pass = password.to_string();
    callbacks.credentials(move |_url, _username_from_url, _allowed_types| {
        Cred::userpass_plaintext(&user, &pass)
    });

    let mut remote = repo.find_remote("origin")?;
    let mut push_options = PushOptions::new();
    push_options.remote_callbacks(callbacks);

    remote.push(
        &[&format!("refs/heads/{}:refs/heads/{}", branch_name, branch_name)],
        Some(&mut push_options),
    )?;

    eprintln!("Successfully pushed branch {}", branch_name);
    Ok(())
}

/// Creates a git tag and pushes it
fn create_and_push_tag(
    repo: &Repository,
    tag_name: &str,
    username: &str,
    password: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let signature = Signature::now(username, TEST_EMAIL)?;

    let head_commit = repo.head()?.peel_to_commit()?;
    repo.tag(
        tag_name,
        head_commit.as_object(),
        &signature,
        &format!("Version {}", tag_name),
        false,
    )?;

    let mut callbacks = RemoteCallbacks::new();
    let user = username.to_string();
    let pass = password.to_string();
    callbacks.credentials(move |_url, _username_from_url, _allowed_types| {
        Cred::userpass_plaintext(&user, &pass)
    });

    let mut remote = repo.find_remote("origin")?;
    let mut push_options = PushOptions::new();
    push_options.remote_callbacks(callbacks);

    remote.push(
        &[&format!("refs/tags/{}:refs/tags/{}", tag_name, tag_name)],
        Some(&mut push_options),
    )?;

    eprintln!("Successfully pushed tag {}", tag_name);
    Ok(())
}

/// Adds a new commit on top of current HEAD
fn add_commit(
    repo: &Repository,
    baker_yaml_content: &str,
    commit_message: &str,
    username: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let signature = Signature::now(username, TEST_EMAIL)?;

    let workdir = repo.workdir().ok_or("No workdir")?;
    fs::write(workdir.join("baker.yaml"), baker_yaml_content)?;

    let mut index = repo.index()?;
    index.add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)?;
    index.write()?;

    let tree_id = index.write_tree()?;
    let tree = repo.find_tree(tree_id)?;
    let parent = repo.head()?.peel_to_commit()?;

    repo.commit(Some("HEAD"), &signature, &signature, commit_message, &tree, &[&parent])?;

    Ok(())
}

/// Pushes the current branch to remote
fn push_current_branch(
    repo: &Repository,
    username: &str,
    password: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut callbacks = RemoteCallbacks::new();
    let user = username.to_string();
    let pass = password.to_string();
    callbacks.credentials(move |_url, _username_from_url, _allowed_types| {
        Cred::userpass_plaintext(&user, &pass)
    });

    let mut remote = repo.find_remote("origin")?;
    let mut push_options = PushOptions::new();
    push_options.remote_callbacks(callbacks);

    let head = repo.head()?;
    let refname = head.name().ok_or("No ref name")?;

    remote.push(&[&format!("{}:{}", refname, refname)], Some(&mut push_options))?;

    Ok(())
}

/// Wait for Gitea API to be ready
fn wait_for_gitea_ready(base_url: &str, client: &Client, timeout_secs: u64) -> bool {
    eprintln!("Waiting for Gitea API at {}", base_url);
    for i in 0..timeout_secs {
        match client.get(&format!("{}/api/v1/version", base_url)).send() {
            Ok(response) => {
                if response.status().is_success() {
                    eprintln!("Gitea API is ready after {} seconds", i);
                    return true;
                }
                eprintln!(
                    "Gitea API responded with status {} (attempt {})",
                    response.status(),
                    i + 1
                );
            }
            Err(e) => {
                eprintln!("Gitea API not ready yet (attempt {}): {}", i + 1, e);
            }
        }
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
    eprintln!("Gitea API failed to become ready after {} seconds", timeout_secs);
    false
}

/// Creates the admin user using testcontainers exec
fn create_admin_user(
    container: &testcontainers::core::Container<GenericImage>,
) -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("Creating admin user via container exec");

    let cmd = ExecCommand::new(vec![
        "gitea",
        "admin",
        "user",
        "create",
        "--username",
        TEST_USER,
        "--password",
        TEST_PASSWORD,
        "--email",
        TEST_EMAIL,
        "--admin",
        "--must-change-password=false",
    ]);

    let mut result = container.exec(cmd)?;

    let exit_code = result.exit_code()?;
    if let Some(code) = exit_code {
        if code != 0 {
            let stderr = result.stderr_to_vec().unwrap_or_default();
            let stderr_str = String::from_utf8_lossy(&stderr);

            if !stderr_str.contains("already exists") {
                return Err(format!(
                    "Failed to create admin user (exit code {}): {}",
                    code, stderr_str
                )
                .into());
            }
        }
    }

    eprintln!("Admin user created successfully");
    Ok(())
}

/// Creates a repository in Gitea via API
fn create_gitea_repo(
    base_url: &str,
    client: &Client,
    repo_name: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    eprintln!("Creating repository: {}", repo_name);

    let create_repo_payload = json!({
        "name": repo_name,
        "description": "Test repository for baker integration tests",
        "private": false,
        "auto_init": false
    });

    let response = client
        .post(&format!("{}/api/v1/user/repos", base_url))
        .basic_auth(TEST_USER, Some(TEST_PASSWORD))
        .json(&create_repo_payload)
        .send()?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().unwrap_or_default();
        return Err(format!("Failed to create repo: {} - {}", status, body).into());
    }

    let repo_url = format!("{}/{}/{}.git", base_url, TEST_USER, repo_name);
    eprintln!("Repository created: {}", repo_url);
    Ok(repo_url)
}

/// Copies a directory recursively
fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
    if !dst.exists() {
        fs::create_dir_all(dst)?;
    }
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

#[test]
#[ignore] // Ignore by default as it requires Docker
fn test_git_clone_from_gitea() {
    let env = get_shared_gitea();

    // Use unique repo name to avoid conflicts with parallel test runs
    let repo_name = "test-template-clone";
    let repo_url = env.create_repo(repo_name).expect("Failed to create repository");

    let template_dir = TempDir::new().expect("Failed to create temp dir");
    create_sample_template(template_dir.path());

    init_and_push_repo(template_dir.path(), &repo_url, TEST_USER, TEST_PASSWORD)
        .expect("Failed to push to Gitea");

    let work_dir = TempDir::new().expect("Failed to create work dir");
    let clone_path = work_dir.path().join("cloned-repo");

    let clone_url = env.clone_url_with_auth(repo_name);

    let _repo = git2::Repository::clone(&clone_url, &clone_path)
        .expect("Failed to clone repository");

    assert!(clone_path.exists(), "Cloned path does not exist");

    assert!(
        clone_path.join("baker.yaml").exists(),
        "baker.yaml not found in cloned repo"
    );
    assert!(
        clone_path.join("README.md.baker.j2").exists(),
        "README.md.baker.j2 not found in cloned repo"
    );
    assert!(
        clone_path.join("src").join("main.txt").exists(),
        "src/main.txt not found in cloned repo"
    );

    let baker_yaml_content = fs::read_to_string(clone_path.join("baker.yaml"))
        .expect("Failed to read baker.yaml");
    assert!(
        baker_yaml_content.contains("schemaVersion: v1"),
        "baker.yaml content mismatch"
    );
}

#[test]
#[ignore] // Ignore by default as it requires Docker
fn test_git_checkout_specific_branch() {
    let env = get_shared_gitea();

    let repo_name = "test-template-branch";
    let repo_url = env.create_repo(repo_name).expect("Failed to create repository");

    let template_dir = TempDir::new().expect("Failed to create temp dir");
    create_sample_template(template_dir.path());

    init_and_push_repo(template_dir.path(), &repo_url, TEST_USER, TEST_PASSWORD)
        .expect("Failed to push to Gitea");

    let local_repo =
        Repository::open(template_dir.path()).expect("Failed to open local repo");

    let feature_content = r#"schemaVersion: v1

questions:
  project_name:
    type: str
    help: Enter project name (feature branch version)
    default: "my feature project"

  version:
    type: str
    help: Enter version
    default: "2.0.0"
"#;

    create_and_push_branch(
        &local_repo,
        "feature",
        feature_content,
        TEST_USER,
        TEST_PASSWORD,
    )
    .expect("Failed to create and push feature branch");

    let work_dir = TempDir::new().expect("Failed to create work dir");
    let clone_path = work_dir.path().join("cloned-feature");

    let clone_url = env.clone_url_with_auth(repo_name);
    let fetch_opts = git2::FetchOptions::new();
    let mut builder = git2::build::RepoBuilder::new();
    builder.fetch_options(fetch_opts);
    builder.branch("feature");

    let repo =
        builder.clone(&clone_url, &clone_path).expect("Failed to clone feature branch");

    let head = repo.head().expect("Failed to get HEAD");
    let branch_name = head.shorthand().unwrap_or("");
    assert_eq!(branch_name, "feature", "Not on feature branch");

    let baker_yaml_content = fs::read_to_string(clone_path.join("baker.yaml"))
        .expect("Failed to read baker.yaml");
    assert!(
        baker_yaml_content.contains("feature branch version"),
        "Content is not from feature branch"
    );
    assert!(
        baker_yaml_content.contains("2.0.0"),
        "Version should be 2.0.0 on feature branch"
    );
}

#[test]
#[ignore] // Ignore by default as it requires Docker
fn test_git_checkout_specific_tag() {
    let env = get_shared_gitea();

    let repo_name = "test-template-tag";
    let repo_url = env.create_repo(repo_name).expect("Failed to create repository");

    let template_dir = TempDir::new().expect("Failed to create temp dir");
    create_sample_template(template_dir.path());

    init_and_push_repo(template_dir.path(), &repo_url, TEST_USER, TEST_PASSWORD)
        .expect("Failed to push to Gitea");

    let local_repo =
        Repository::open(template_dir.path()).expect("Failed to open local repo");

    create_and_push_tag(&local_repo, "v1.0.0", TEST_USER, TEST_PASSWORD)
        .expect("Failed to create and push tag");

    let new_content = r#"schemaVersion: v1

questions:
  project_name:
    type: str
    help: Enter project name (after v1.0.0)
    default: "my newer project"
"#;

    add_commit(&local_repo, new_content, "Post v1.0.0 changes", TEST_USER)
        .expect("Failed to add commit");

    push_current_branch(&local_repo, TEST_USER, TEST_PASSWORD)
        .expect("Failed to push changes");

    let work_dir = TempDir::new().expect("Failed to create work dir");
    let clone_path = work_dir.path().join("cloned-tag");

    let clone_url = env.clone_url_with_auth(repo_name);
    let repo = git2::Repository::clone(&clone_url, &clone_path)
        .expect("Failed to clone repository");

    let tag_ref = repo.find_reference("refs/tags/v1.0.0").expect("Failed to find tag");
    let tag_commit = tag_ref.peel_to_commit().expect("Failed to peel tag to commit");

    repo.checkout_tree(tag_commit.as_object(), None).expect("Failed to checkout tag");
    repo.set_head_detached(tag_commit.id()).expect("Failed to set HEAD to tag");

    let baker_yaml_content = fs::read_to_string(clone_path.join("baker.yaml"))
        .expect("Failed to read baker.yaml");
    assert!(
        baker_yaml_content.contains("my project"),
        "Content should be from v1.0.0 tag"
    );
    assert!(
        !baker_yaml_content.contains("after v1.0.0"),
        "Content should not include post-tag changes"
    );
}

#[test]
#[ignore] // Ignore by default as it requires Docker
fn test_jsonschema_file_template_from_gitea() {
    let env = get_shared_gitea();

    let repo_name = "jsonschema-file-template";
    let repo_url = env.create_repo(repo_name).expect("Failed to create repository");

    let template_dir = TempDir::new().expect("Failed to create temp dir");
    let manifest_dir =
        std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let source_template =
        Path::new(&manifest_dir).join("tests/templates/jsonschema_file");
    copy_dir_recursive(&source_template, template_dir.path())
        .expect("Failed to copy jsonschema_file template");

    assert!(
        template_dir.path().join("baker.yaml").exists(),
        "baker.yaml should exist in copied template"
    );
    assert!(
        template_dir.path().join("database.schema.json").exists(),
        "database.schema.json should exist in copied template"
    );

    init_and_push_repo(template_dir.path(), &repo_url, TEST_USER, TEST_PASSWORD)
        .expect("Failed to push to Gitea");

    let work_dir = TempDir::new().expect("Failed to create work dir");
    let output_dir = work_dir.path().join("output");
    fs::create_dir_all(&output_dir).expect("Failed to create output dir");

    let clone_url = env.clone_url_with_auth(repo_name);

    let args = Args {
        template: clone_url,
        output_dir: output_dir.clone(),
        force: true,
        verbose: 2,
        answers: None,
        skip_confirms: vec![All],
        non_interactive: true,
        dry_run: false,
    };

    run(args).expect("Baker run failed");

    let manifest_dir =
        std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let expected_dir = Path::new(&manifest_dir).join("tests/expected/jsonschema_file");

    let output_config = output_dir.join("config.toml");
    assert!(output_config.exists(), "config.toml should be generated");

    let output_content =
        fs::read_to_string(&output_config).expect("Failed to read output config.toml");
    let expected_content = fs::read_to_string(expected_dir.join("config.toml"))
        .expect("Failed to read expected config.toml");

    assert_eq!(
        output_content.trim(),
        expected_content.trim(),
        "Generated config.toml should match expected output"
    );

    assert!(
        !output_dir.join("database.schema.json").exists(),
        "database.schema.json should be ignored by .bakerignore"
    );

    eprintln!("Successfully generated project from jsonschema_file template via Gitea!");
}
