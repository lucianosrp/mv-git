use std::env;
use std::error::Error;
use std::fs::File;
use std::fs::{self, DirEntry};
use std::io::{self, BufRead, ErrorKind};
use std::path::Path;

fn read_gitignore(file: &Path) -> io::Result<Vec<String>> {
    let file = File::open(file)?;
    let reader = io::BufReader::new(file);
    let mut lines: Vec<String> = Vec::new();
    for line in reader.lines() {
        let line = line?;
        lines.push(line.replace("/", "").trim().to_string())
    }
    Ok(lines)
}

fn copy_entry(entry: &DirEntry, dst: &Path, gitignore: &Option<Vec<String>>) -> io::Result<()> {
    let file_type = entry.file_type()?;
    let dest_path = dst.join(entry.file_name());

    if file_type.is_dir() {
        copy_dir_recursive(&entry.path(), &dest_path, gitignore)?;
    } else {
        fs::copy(&entry.path(), &dest_path)?;
    }
    Ok(())
}

fn copy_dir_recursive(src: &Path, dst: &Path, gitignore: &Option<Vec<String>>) -> io::Result<()> {
    if !dst.exists() {
        fs::create_dir_all(dst)?;
    }

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let entry_name = entry.file_name().to_string_lossy().into_owned();
        if gitignore
            .as_ref()
            .map_or(true, |gi| !gi.contains(&entry_name))
        {
            copy_entry(&entry, dst, gitignore)?;
        }
    }
    Ok(())
}

fn move_dir(src: &Path, dst: &Path, gitignore: &Option<Vec<String>>, copy: bool) -> io::Result<()> {
    if !src.exists() {
        return Err(io::Error::new(
            ErrorKind::NotFound,
            "Source directory not found",
        ));
    }

    // Handle potential errors during the copy process
    if let Err(e) = copy_dir_recursive(src, dst, gitignore) {
        eprintln!("Error copying directory: {}", e);
        return Err(e); // Propagate the error
    }

    if !copy {
        if let Err(e) = fs::remove_dir_all(src) {
            eprintln!("Error removing source directory: {}", e);
            return Err(e); // Propagate the error
        }
    }

    Ok(())
}

fn is_git_dir(path: &Path) -> io::Result<(bool, Option<Vec<String>>)> {
    let mut is_dir = false;
    let mut git_ignore = None;

    for entry in path.read_dir()? {
        let entry = entry?;
        match entry.file_name().to_str() {
            Some(".git") => is_dir = true,
            Some(".gitignore") => git_ignore = Some(read_gitignore(&entry.path())?),
            _ => (),
        }
    }
    Ok((is_dir, git_ignore))
}

fn parse_args(args: Vec<String>) -> Result<(String, String, bool), Box<dyn Error>> {
    if args.len() < 3 {
        return Err("Usage: <source> <destination> [--copy | -c]".into());
    }
    let source = args[1].clone();
    let dest = args[2].clone();
    let copy = args
        .get(3)
        .map_or(false, |arg| arg == "--copy" || arg == "-c");

    Ok((source, dest, copy))
}

fn move_recursive(path: &Path, dst: String, copy: bool) -> io::Result<()> {
    if path.exists() && path.is_dir() {
        for entry in path.read_dir()? {
            let entry_path = entry?.path();
            let path_name = entry_path
                .canonicalize()
                .unwrap()
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .to_owned();

            let (is_git, gitignore) = is_git_dir(&entry_path)?;
            let new_dest_path = &Path::new(&dst).join(path_name.clone());

            if is_git {
                move_dir(&entry_path, &new_dest_path, &gitignore, copy)?;
            } else {
                println!("{:?} is not a git dir!", path)
            }
        }
    } else {
        println!("{:?} is not a dir or does not exists", path)
    }
    Ok(())
}

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();
    let (fp, dest, copy) = parse_args(args).unwrap();

    let p = Path::new(&fp);
    move_recursive(p, dest, copy)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_read_gitignore() -> io::Result<()> {
        let dir = tempdir()?;
        let file_path = dir.path().join(".gitignore");
        {
            let mut file = File::create(&file_path)?;
            writeln!(file, "target/")?;
            writeln!(file, "node_modules/")?;
        }

        let gitignore = read_gitignore(&file_path)?;
        assert_eq!(gitignore, vec!["target", "node_modules"]);
        Ok(())
    }

    #[test]
    fn test_copy_dir_recursive() -> io::Result<()> {
        let src_dir = tempdir()?;
        let dst_dir = tempdir()?;

        let gitignore = Some(vec!["ignored".to_string()]);

        // Create some files and directories in the source directory
        File::create(src_dir.path().join("file1.txt"))?;
        fs::create_dir(src_dir.path().join("ignored"))?;
        File::create(src_dir.path().join("ignored").join("file2.txt"))?;

        copy_dir_recursive(src_dir.path(), dst_dir.path(), &gitignore)?;

        // Check that file1.txt exists in the destination
        assert!(dst_dir.path().join("file1.txt").exists());
        // Check that the ignored directory does not exist in the destination
        assert!(!dst_dir.path().join("ignored").exists());

        Ok(())
    }

    #[test]
    fn test_move_dir() -> io::Result<()> {
        let src_dir = tempdir()?;
        let dst_dir = tempdir()?.path().join("moved");

        let gitignore = Some(vec!["ignored".to_string()]);

        // Create some files and directories in the source directory
        File::create(src_dir.path().join("file1.txt"))?;
        fs::create_dir(src_dir.path().join("ignored"))?;
        File::create(src_dir.path().join("ignored").join("file2.txt"))?;

        move_dir(src_dir.path(), &dst_dir, &gitignore, false)?;

        // Check that the source directory is removed
        assert!(!src_dir.path().exists());
        // Check that file1.txt exists in the destination
        assert!(dst_dir.join("file1.txt").exists());
        // Check that the ignored directory does not exist in the destination
        assert!(!dst_dir.join("ignored").exists());

        Ok(())
    }

    #[test]
    fn test_is_git_dir() -> io::Result<()> {
        let dir = tempdir()?;
        File::create(dir.path().join(".gitignore"))?;
        fs::create_dir(dir.path().join(".git"))?;

        let (is_git, gitignore) = is_git_dir(dir.path())?;

        assert!(is_git);
        assert!(gitignore.is_some());

        Ok(())
    }
}
