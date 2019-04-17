use amethyst::{assets::Source, Error};
use std::{
    convert::TryInto,
    fs::File,
    io::{self, Read, Seek},
    path::Path,
    sync::Mutex,
    time::SystemTime,
};
use zip::{
    read::{AsciiLowercaseStr, AsciiLowercaseString},
    ZipArchive,
};

pub struct Pk3Source<R: Read + Seek = File> {
    modified: u64,
    inner: Mutex<ZipArchive<R, AsciiLowercaseString>>,
}

impl<R: Read + Seek> Pk3Source<R> {
    pub fn new(reader: R) -> io::Result<Self> {
        Ok(Pk3Source {
            modified: 0,
            inner: ZipArchive::new(reader)?.into(),
        })
    }
}

impl Pk3Source {
    pub fn open(path: &impl AsRef<Path>) -> io::Result<Self> {
        let file = File::open(path)?;
        let modified = file
            .metadata()?
            .modified()?
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_err(|_| io::ErrorKind::Other)?
            .as_secs();

        Ok(Pk3Source {
            modified,
            inner: ZipArchive::new(file)?.into(),
        })
    }
}

impl Source for Pk3Source {
    fn modified(&self, _path: &str) -> Result<u64, Error> {
        Ok(self.modified)
    }

    fn load(&self, path: &str) -> Result<Vec<u8>, Error> {
        let mut inner = self.inner.lock().expect("Panicked while lock was held");

        let path_lowercase_owned = AsciiLowercaseString::from(path.to_string());
        let path_lowercase: &AsciiLowercaseStr = path_lowercase_owned.as_ref();

        if let Ok(mut file) = inner.by_name(path_lowercase) {
            let mut out = Vec::with_capacity(file.size().try_into().map_err(Error::new)?);

            file.read_to_end(&mut out).map_err(|err| Error::new(err))?;

            return Ok(out);
        }

        // The weird `if .. return` instead of `if..else` and duplicated read-to-vec logic are because
        // otherwise we get lifetime errors.

        let path_as_path: &Path = path.as_ref();

        if path_as_path.extension().is_some() {
            return Err(Error::new(io::Error::from(io::ErrorKind::NotFound)));
        }

        let file_name = path_as_path.file_stem();

        let index = {
            let mut iter = inner.starting_with(path_lowercase);
            loop {
                let (name, index) = if let Some(kv) = iter.next() {
                    kv
                } else {
                    return Err(Error::new(io::Error::from(io::ErrorKind::NotFound)));
                };

                let name: &Path = name.as_ref();

                if name.file_stem() == file_name {
                    break index;
                }
            }
        };

        if let Ok(mut file) = inner.by_index(index) {
            let mut out = Vec::with_capacity(file.size().try_into().map_err(Error::new)?);

            file.read_to_end(&mut out).map_err(|err| Error::new(err))?;

            return Ok(out);
        }

        Err(Error::new(io::Error::from(io::ErrorKind::NotFound)))
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
