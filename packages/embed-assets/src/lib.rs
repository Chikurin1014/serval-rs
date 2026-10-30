//! An app's assets, appended to its binary so it is one file.
//!
//! After the binary, each file is its path's length (`u32`), its path, its
//! data's length (`u64`) and its data; then all of their length (`u64`) and
//! [`MAGIC`]. Numbers are little-endian.

use std::{
    collections::HashMap,
    fs::{self, File},
    io::{self, Read, Seek, SeekFrom, Write},
    ops::Range,
    path::Path,
};

/// The end of a binary with assets.
pub const MAGIC: &[u8; 8] = b"SRVLASTS";
const TRAILER_LEN: u64 = 16;

/// Assets read from a binary, by their paths (e.g. `assets/main.css`).
pub struct Assets {
    data: Vec<u8>,
    files: HashMap<String, Range<usize>>,
}

impl Assets {
    /// The assets after the binary at `path`, if any.
    pub fn read(path: &Path) -> io::Result<Option<Self>> {
        let mut file = File::open(path)?;
        let Some(len) = payload_len(&mut file)? else {
            return Ok(None);
        };
        file.seek(SeekFrom::End(-((len + TRAILER_LEN) as i64)))?;
        let mut data = vec![0; len as usize];
        file.read_exact(&mut data)?;
        let files = index(&data).ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidData, "Broken assets in the binary")
        })?;
        Ok(Some(Self { data, files }))
    }

    /// The data of the file at `path`.
    pub fn get(&self, path: &str) -> Option<&[u8]> {
        self.files.get(path).map(|range| &self.data[range.clone()])
    }

    /// How many files there are.
    pub fn len(&self) -> usize {
        self.files.len()
    }

    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }
}

/// Writes the binary at `binary` to `out`, with the files in `root`'s `dir`
/// after it. A binary which has assets already is refused.
pub fn embed(binary: &Path, root: &Path, dir: &str, out: &Path) -> io::Result<usize> {
    let mut source = File::open(binary)?;
    if payload_len(&mut source)?.is_some() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "The binary has assets already",
        ));
    }
    source.rewind()?;
    let mut files = Vec::new();
    walk(&root.join(dir), dir, &mut files)?;
    files.sort();

    let mut out = io::BufWriter::new(File::create(out)?);
    io::copy(&mut source, &mut out)?;
    let mut len = 0;
    for (path, file) in &files {
        let data = fs::read(file)?;
        out.write_all(&(path.len() as u32).to_le_bytes())?;
        out.write_all(path.as_bytes())?;
        out.write_all(&(data.len() as u64).to_le_bytes())?;
        out.write_all(&data)?;
        len += 4 + path.len() as u64 + 8 + data.len() as u64;
    }
    out.write_all(&len.to_le_bytes())?;
    out.write_all(MAGIC)?;
    out.flush()?;
    Ok(files.len())
}

/// The files under `dir`, by their paths from the root, as `/`-separated.
fn walk(dir: &Path, path: &str, files: &mut Vec<(String, std::path::PathBuf)>) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let name = entry.file_name();
        let name = name.to_str().ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidData, "An asset's name isn't UTF-8")
        })?;
        let entry_path = format!("{path}/{name}");
        if entry.file_type()?.is_dir() {
            walk(&entry.path(), &entry_path, files)?;
        } else {
            files.push((entry_path, entry.path()));
        }
    }
    Ok(())
}

/// The assets' length, if the file ends with them.
fn payload_len(file: &mut File) -> io::Result<Option<u64>> {
    let size = file.seek(SeekFrom::End(0))?;
    if size < TRAILER_LEN {
        return Ok(None);
    }
    file.seek(SeekFrom::End(-(TRAILER_LEN as i64)))?;
    let mut trailer = [0; TRAILER_LEN as usize];
    file.read_exact(&mut trailer)?;
    let (len, magic) = trailer.split_at(8);
    if magic != MAGIC {
        return Ok(None);
    }
    let len = u64::from_le_bytes(len.try_into().unwrap());
    Ok((len <= size - TRAILER_LEN).then_some(len))
}

/// Where each file's data is in `data`; `None` if it is broken.
fn index(data: &[u8]) -> Option<HashMap<String, Range<usize>>> {
    let mut files = HashMap::new();
    let mut at = 0;
    while at < data.len() {
        let path_len = u32::from_le_bytes(data[take(data, &mut at, 4)?].try_into().ok()?);
        let path = &data[take(data, &mut at, path_len as usize)?];
        let path = std::str::from_utf8(path).ok()?.to_string();
        let data_len = u64::from_le_bytes(data[take(data, &mut at, 8)?].try_into().ok()?);
        files.insert(path, take(data, &mut at, data_len as usize)?);
    }
    Some(files)
}

/// The next `len` bytes of `data` from `at`, moving it past them.
fn take(data: &[u8], at: &mut usize, len: usize) -> Option<Range<usize>> {
    let range = *at..at.checked_add(len).filter(|end| *end <= data.len())?;
    *at = range.end;
    Some(range)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_files_embedded_once() {
        let root = std::env::temp_dir().join(format!("embed-assets-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("assets/fonts")).unwrap();
        fs::write(root.join("app"), b"binary").unwrap();
        fs::write(root.join("assets/main.css"), b"body {}").unwrap();
        fs::write(root.join("assets/fonts/a.woff2"), [0, 1, 2]).unwrap();
        fs::write(root.join("assets/empty.js"), b"").unwrap();

        // None in the binary itself
        assert!(Assets::read(&root.join("app")).unwrap().is_none());
        let out = root.join("app-embedded");
        assert_eq!(embed(&root.join("app"), &root, "assets", &out).unwrap(), 3);
        let assets = Assets::read(&out).unwrap().unwrap();
        assert_eq!(assets.len(), 3);
        assert_eq!(assets.get("assets/main.css"), Some(&b"body {}"[..]));
        assert_eq!(assets.get("assets/fonts/a.woff2"), Some(&[0, 1, 2][..]));
        assert_eq!(assets.get("assets/empty.js"), Some(&[][..]));
        assert_eq!(assets.get("assets/none.css"), None);
        // The binary first, as it was
        assert!(fs::read(&out).unwrap().starts_with(b"binary"));

        let again = root.join("app-again");
        assert!(embed(&out, &root, "assets", &again).is_err());
        fs::remove_dir_all(&root).unwrap();
    }
}
