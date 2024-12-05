use baker::cli::Args;
use clap::Parser;
use std::ffi::OsString;
use std::path::PathBuf;

fn make_args(args: &[&str]) -> Vec<OsString> {
    let mut res = vec![OsString::from("baker")];
    res.extend(args.iter().map(OsString::from));
    res
}

#[test]
fn test_basic_args() {
    let args = make_args(&["./template", "./output"]);
    let parsed = Args::try_parse_from(args).unwrap();

    assert_eq!(parsed.template, "./template");
    assert_eq!(parsed.output_dir, PathBuf::from("./output"));
    assert!(!parsed.force);
    assert!(!parsed.verbose);
    assert!(!parsed.skip_hooks_check);
}

#[test]
fn test_all_flags() {
    let args = make_args(&[
        "--force",
        "--verbose",
        "--skip-hooks-check",
        "./template",
        "./output",
    ]);
    let parsed = Args::try_parse_from(args).unwrap();

    assert!(parsed.force);
    assert!(parsed.verbose);
    assert!(parsed.skip_hooks_check);
}

#[test]
fn test_short_flags() {
    let args = make_args(&["-f", "-v", "./template", "./output"]);
    let parsed = Args::try_parse_from(args).unwrap();

    assert!(parsed.force);
    assert!(parsed.verbose);
}

#[test]
fn test_git_url_template() {
    let args = make_args(&["https://github.com/user/template.git", "./output"]);
    let parsed = Args::try_parse_from(args).unwrap();

    assert_eq!(parsed.template, "https://github.com/user/template.git");
}

#[test]
fn test_missing_args() {
    let args = make_args(&["./template"]);
    assert!(Args::try_parse_from(args).is_err());
}

#[test]
fn test_too_many_args() {
    let args = make_args(&["./template", "./output", "extra"]);
    assert!(Args::try_parse_from(args).is_err());
}
