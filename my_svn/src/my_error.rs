#[allow(clippy::enum_variant_names)] //da warning ca toate campurile contin Error in nume
pub enum MyCostumError {
    IOError(std::io::Error),
    DBError(rusqlite::Error),
    IGNORError(ignore::Error),
    WalkDirError(walkdir::Error),
    MyError(String),
}

impl std::fmt::Debug for MyCostumError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MyCostumError::IOError(err) => write!(f, "IO Error: {:?}", err),
            MyCostumError::DBError(err) => write!(f, "Database Error: {:?}", err),
            MyCostumError::IGNORError(err) => write!(f, "Ignore Error: {:?}", err),
            MyCostumError::WalkDirError(err) => write!(f, "WalkDir Error: {:?}", err),
            MyCostumError::MyError(msg) => write!(f, "Custom Error: {}", msg),
        }
    }
}
impl std::fmt::Display for MyCostumError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")?;
        Ok(())
    }
}

impl From<std::io::Error> for MyCostumError {
    fn from(value: std::io::Error) -> Self {
        Self::IOError(value)
    }
}

impl From<rusqlite::Error> for MyCostumError {
    fn from(value: rusqlite::Error) -> Self {
        Self::DBError(value)
    }
}
impl From<ignore::Error> for MyCostumError {
    fn from(value: ignore::Error) -> Self {
        Self::IGNORError(value)
    }
}
impl From<String> for MyCostumError {
    fn from(value: String) -> Self {
        Self::MyError(value)
    }
}
impl From<walkdir::Error> for MyCostumError {
    fn from(value: walkdir::Error) -> Self {
        Self::WalkDirError(value)
    }
}
