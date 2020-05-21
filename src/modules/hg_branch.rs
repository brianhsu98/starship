use unicode_segmentation::UnicodeSegmentation;

use super::{Context, Module, RootModuleConfig};

use crate::configs::hg_branch::HgBranchConfig;

use std::path::PathBuf;

/// Creates a module with the Hg bookmark or branch in the current directory
///
/// Will display the bookmark or branch name if the current directory is an hg repo
pub fn module<'a>(context: &'a Context) -> Option<Module<'a>> {
    // My own hack stacked on top to find the hg directory recursively.
    let hg_path = match find_hg_directory(context.current_dir.clone()) {
        Some(hg_path) => hg_path,
        None => return None
    };

    let mut module = context.new_module("hg_branch");
    let config = HgBranchConfig::try_load(module.config);
    module.set_style(config.style);

    module.get_prefix().set_value("on ");

    module.create_segment("symbol", &config.symbol);

    // TODO: Once error handling is implemented, warn the user if their config
    // truncation length is nonsensical
    let len = if config.truncation_length <= 0 {
        log::warn!(
            "\"truncation_length\" should be a positive value, found {}",
            config.truncation_length
        );
        std::usize::MAX
    } else {
        config.truncation_length as usize
    };

    let branch_name =
        get_hg_current_bookmark(hg_path.clone()).unwrap_or_else(|| get_hg_commit_name(hg_path));

    let truncated_graphemes = get_graphemes(&branch_name, len);
    // The truncation symbol should only be added if we truncated
    let truncated_and_symbol = if len < graphemes_len(&branch_name) {
        let truncation_symbol = get_graphemes(config.truncation_symbol, 1);
        truncated_graphemes + &truncation_symbol
    } else {
        truncated_graphemes
    };

    module.create_segment(
        "name",
        &config.branch_name.with_value(&truncated_and_symbol),
    );

    Some(module)
}

/// Recursively ascends through the current path until either the root is reached or
/// a .hg directory is found.
fn find_hg_directory(mut current_path: PathBuf) -> Option<PathBuf> {
    while current_path != PathBuf::new() {
        let read_dir = match current_path.read_dir() {
            Ok(read_dir) => read_dir,
            Err(_e) => return None
        };

        for direntry in read_dir {
            let entry = match direntry {
                Ok(entry) => entry,
                Err(_e) => return None,
            };

            let file_type = match entry.file_type() {
                Ok(file_type) => file_type,
                Err(_e) => return None,
            };

            if file_type.is_dir() && entry.file_name() == ".hg" {
                return Some(entry.path());
            }
        }
        current_path.pop();
    }
    None
}

fn get_hg_commit_name(hg_path: PathBuf) -> String {
    // This is reading the entire namejournal file, which is somewhat large. Faster than running hg id, though.
    let namejournal = std::fs::read_to_string(hg_path.join("namejournal"))
        .map(|s| s.trim().into())
        .unwrap_or_else(|_| "".to_string());
    let lines: Vec<&str> = namejournal.split("\n").collect();

    match lines.last() {
        Some(line) => line.to_owned().to_string(),
        None => "".to_string()
    }
}

fn get_hg_current_bookmark(hg_path: PathBuf) -> Option<String> {
    std::fs::read_to_string(hg_path.join("bookmarks.current"))
        .map(|s| s.trim().into())
        .ok()
}

fn get_graphemes(text: &str, length: usize) -> String {
    UnicodeSegmentation::graphemes(text, true)
        .take(length)
        .collect::<Vec<&str>>()
        .concat()
}

fn graphemes_len(text: &str) -> usize {
    UnicodeSegmentation::graphemes(&text[..], true).count()
}
