use std::{
    fs::{File as StdFile},
    io,
    os::unix::fs::PermissionsExt,
    path::{Path},
};
use zip::write::FileOptions;
use walkdir::WalkDir;

pub fn zip_any(src_path: &Path, zip_path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let zip_file = StdFile::create(zip_path)?;
    let mut zip = zip::ZipWriter::new(zip_file);
    let options: FileOptions<'_, ()> = FileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    if src_path.is_file() {
        let file_name = src_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy();

        adjust_permissions(src_path)?;
        let mut f = StdFile::open(src_path)?;
        zip.start_file(file_name, options)?;
        io::copy(&mut f, &mut zip)?;
    } else {
        for entry in WalkDir::new(src_path).into_iter().filter_map(Result::ok) {
            let path = entry.path();
            let name = path.strip_prefix(src_path)?;

            if path.is_file() {
                adjust_permissions(path)?;
                let mut f = StdFile::open(path)?;
                zip.start_file(name.to_string_lossy(), options)?;
                io::copy(&mut f, &mut zip)?;
            } else if name.as_os_str().len() != 0 {
                zip.add_directory(name.to_string_lossy(), options)?;
            }
        }
    }

    zip.finish()?;
    Ok(())
}

fn adjust_permissions(path: &Path) -> io::Result<()> {
    if let Ok(metadata) = path.metadata() {
        let mut perms = metadata.permissions();
        let mode = perms.mode();

        if mode & 0o400 == 0 {
            perms.set_mode(mode | 0o400);
            std::fs::set_permissions(path, perms)?;
        }
    }
    Ok(())
}
