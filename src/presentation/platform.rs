use crate::error::AppResult;
use base64::{engine::general_purpose::STANDARD, Engine};
use gtk::prelude::*;
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
};

pub(super) fn read_limited(path: &Path, limit: usize) -> AppResult<Vec<u8>> {
    let file = fs::File::open(path).map_err(|error| error.to_string())?;
    if file.metadata().map_err(|error| error.to_string())?.len() > limit as u64 {
        return Err("File size limit exceeded".into());
    }
    let mut bytes = Vec::new();
    file.take(limit as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| error.to_string())?;
    if bytes.len() > limit {
        return Err("File size limit exceeded".into());
    }
    Ok(bytes)
}

pub(super) fn local_data_uri(uri: &str, limit: usize) -> Option<String> {
    let path = if uri.starts_with("file://") {
        url::Url::parse(uri).ok()?.to_file_path().ok()?
    } else {
        PathBuf::from(uri)
    };
    if !path.is_absolute() {
        return None;
    }
    let bytes = read_limited(&path, limit).ok()?;
    let mime = match path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_lowercase()
        .as_str()
    {
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "gif" => "image/gif",
        "ttf" | "bin" => "font/ttf",
        "otf" => "font/otf",
        _ => "image/png",
    };
    Some(format!("data:{mime};base64,{}", STANDARD.encode(bytes)))
}

pub(super) fn clipboard(text: &str) {
    gtk::Clipboard::get(&gtk::gdk::SELECTION_CLIPBOARD).set_text(text);
}

pub(super) fn choose_file(
    parent: &gtk::Window,
    action: gtk::FileChooserAction,
    title: &str,
    patterns: &[&str],
    name: Option<&str>,
) -> Option<PathBuf> {
    let dialog = gtk::FileChooserDialog::new(Some(title), Some(parent), action);
    dialog.add_buttons(&[
        ("Cancel", gtk::ResponseType::Cancel),
        (
            if action == gtk::FileChooserAction::Save {
                "Save"
            } else {
                "Open"
            },
            gtk::ResponseType::Accept,
        ),
    ]);
    dialog.set_modal(true);
    if let Some(name) = name {
        dialog.set_current_name(name);
        dialog.set_do_overwrite_confirmation(true);
    }
    if !patterns.is_empty() {
        let filter = gtk::FileFilter::new();
        filter.set_name(Some(title));
        for pattern in patterns {
            filter.add_pattern(pattern);
        }
        dialog.add_filter(filter);
    }
    let path = (dialog.run() == gtk::ResponseType::Accept)
        .then(|| dialog.filename())
        .flatten();
    dialog.close();
    path
}
