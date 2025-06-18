use super::{answers::AnswerProcessor, hooks::HookManager, Args};
use crate::{
    config::Config,
    dialoguer::confirm,
    error::{Error, Result},
    ignore::parse_bakerignore_file,
    ioutils::{copy_file, create_dir_all, get_output_dir, write_file},
    loader::TemplateSource,
    renderer::{MiniJinjaRenderer, TemplateRenderer},
    template::{operation::TemplateOperation, processor::TemplateProcessor},
};
use walkdir::WalkDir;

pub fn run(args: Args) -> Result<()> {
    let engine: Box<dyn TemplateRenderer> = Box::new(MiniJinjaRenderer::new());

    let output_root = get_output_dir(args.output_dir.clone(), args.force)?;

    let template_root = TemplateSource::from_string(
        args.template.as_str(),
        args.should_skip_overwrite_confirms(),
    )?;

    let config = Config::load_config(&template_root)?;
    let Config::V1(config_v1) = config;

    // Setup hook manager
    let hook_manager =
        HookManager::new(engine.as_ref(), args.should_skip_hook_confirms());
    let (pre_hook_filename, post_hook_filename) = hook_manager.render_hook_filenames(
        &config_v1.pre_hook_filename,
        &config_v1.post_hook_filename,
    )?;

    let (execute_hooks, pre_hook_file, post_hook_file) = hook_manager.prepare_hooks(
        &template_root,
        &pre_hook_filename,
        &post_hook_filename,
    )?;

    // Execute pre-generation hook
    let pre_hook_stdout = hook_manager.execute_pre_hook(
        &template_root,
        &output_root,
        &pre_hook_file,
        execute_hooks,
    )?;

    // Setup answer processor and collect answers
    let answer_processor = AnswerProcessor::new(engine.as_ref(), args.non_interactive);
    let initial_answers =
        answer_processor.get_initial_answers(args.answers.clone(), pre_hook_stdout)?;
    let answers =
        answer_processor.collect_answers(&Config::V1(config_v1), initial_answers)?;

    // Process template files
    process_template_files(&args, &engine, &template_root, &output_root, &answers)?;

    // Execute post-generation hook
    hook_manager.execute_post_hook(
        &template_root,
        &output_root,
        &post_hook_file,
        execute_hooks,
        Some(&answers),
    )?;

    println!("Template generation completed successfully in {}.", output_root.display());
    Ok(())
}

fn process_template_files(
    args: &Args,
    engine: &Box<dyn TemplateRenderer>,
    template_root: &std::path::PathBuf,
    output_root: &std::path::PathBuf,
    answers: &serde_json::Value,
) -> Result<()> {
    // Process ignore patterns
    let bakerignore = parse_bakerignore_file(template_root)?;

    let processor = TemplateProcessor::new(
        engine.as_ref(),
        template_root,
        output_root,
        answers,
        &bakerignore,
    );

    // Process template files
    for dir_entry in WalkDir::new(template_root) {
        let template_entry = dir_entry?.path().to_path_buf();
        match processor.process(&template_entry) {
            Ok(file_operation) => {
                let user_confirmed_overwrite =
                    handle_file_operation(args, &file_operation)?;
                let message = file_operation.get_message(user_confirmed_overwrite);
                log::info!("{}", message);
            }
            Err(e) => match e {
                Error::ProcessError { .. } => log::warn!("{}", e),
                _ => log::error!("{}", e),
            },
        }
    }

    Ok(())
}

fn handle_file_operation(
    args: &Args,
    file_operation: &TemplateOperation,
) -> Result<bool> {
    match file_operation {
        TemplateOperation::Write { target, target_exists, .. }
        | TemplateOperation::Copy { target, target_exists, .. } => {
            let skip_prompt = args.should_skip_overwrite_confirms() || !target_exists;
            let user_confirmed =
                confirm(skip_prompt, format!("Overwrite {}?", target.display()))?;

            if user_confirmed {
                match file_operation {
                    TemplateOperation::Copy { source, target, .. } => {
                        copy_file(source, target)?;
                    }
                    TemplateOperation::Write { content, target, .. } => {
                        write_file(content, target)?;
                    }
                    _ => unreachable!(),
                }
            }
            Ok(user_confirmed)
        }
        TemplateOperation::CreateDirectory { target, target_exists } => {
            if !target_exists {
                create_dir_all(target)?;
            }
            Ok(true)
        }
        TemplateOperation::Ignore { .. } => Ok(true),
    }
}
