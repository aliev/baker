use baker::loader::get_template;
use baker::loader::git::GitLoader;
use baker::loader::interface::TemplateLoader;
use baker::loader::local::LocalLoader;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

#[test]
fn test_local_loader_existing_directory() {
    // Create a temporary directory with some content
    let temp_dir = TempDir::new().unwrap();
    let template_path = temp_dir.path().join("test_template");
    fs::create_dir(&template_path).unwrap();
    fs::write(template_path.join("baker.yaml"), "project_name: test").unwrap();

    // Test loading existing template
    let loader = LocalLoader::new(&template_path);
    let result = loader.load();
    assert!(result.is_ok());
    assert_eq!(result.unwrap().0, template_path);
}

#[test]
fn test_local_loader_non_existing_directory() {
    let non_existing_path = PathBuf::from("/path/that/does/not/exist");
    let loader = LocalLoader::new(&non_existing_path);
    let result = loader.load();
    assert!(result.is_err());
}

#[test]
fn test_template_source_from_string_local_path() {
    // Create a temporary directory
    let temp_dir = TempDir::new().unwrap();
    let template_path = temp_dir.path().join("test_template");
    fs::create_dir(&template_path).unwrap();
    fs::write(template_path.join("baker.yaml"), "project_name: test").unwrap();

    // Test that local paths are correctly identified and loaded
    let result = get_template(template_path.to_str().unwrap(), true);
    assert!(result.is_ok());
    assert_eq!(result.unwrap().0, template_path);
}

#[test]
fn test_template_source_from_string_invalid_local_path() {
    let result = get_template("/path/that/does/not/exist", true);
    assert!(result.is_err());
}

/// Test that SSH URLs are correctly identified as Git URLs
#[test]
fn test_ssh_url_identification() {
    let ssh_urls = vec![
        "git@github.com:user/repo",
        "git@github.com:user/repo.git",
        "user@gitlab.com:group/project",
        "git@bitbucket.org:team/repository",
    ];

    for url in ssh_urls {
        assert!(
            GitLoader::<&str>::is_git_url(url),
            "Failed to identify {url} as git URL"
        );
    }
}

/// Test that HTTPS URLs are correctly identified as Git URLs
#[test]
fn test_https_url_identification() {
    let https_urls = vec![
        "https://github.com/user/repo",
        "https://github.com/user/repo.git",
        "https://gitlab.com/group/project",
        "https://bitbucket.org/team/repository",
    ];

    for url in https_urls {
        assert!(
            GitLoader::<&str>::is_git_url(url),
            "Failed to identify {url} as git URL"
        );
    }
}

/// Test that local paths are NOT identified as Git URLs
#[test]
fn test_local_path_identification() {
    let local_paths = vec![
        "/path/to/template",
        "./relative/path",
        "../parent/directory",
        "simple_name",
        "C:\\Windows\\Path",
    ];

    for path in local_paths {
        assert!(
            !GitLoader::<&str>::is_git_url(path),
            "Incorrectly identified {path} as git URL"
        );
    }
}
