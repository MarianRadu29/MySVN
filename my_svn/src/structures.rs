pub fn get_parent_name() -> Result<String, std::io::Error> {
    Ok(format!(
        "{}",
        std::env::current_dir()?.parent().unwrap().display() //de cele mai multe ori am un parinte,deci e ok daca las unwrap
    ))
}
pub fn get_default_ignores() -> Result<Vec<String>, std::io::Error> {
    let result = vec![
        get_parent_name()? + ("\\my_svn"),
        get_parent_name()? + ("\\svn_ignore"),
        //ce e mai jos e in cazul in care se face clone la tot repo ul 
        get_parent_name()? + ("\\.git"),
        get_parent_name()? + ("\\README.md"),
        get_parent_name()? + ("\\.gitignore"),
    ];
    Ok(result)
}

#[derive(Debug, Clone)]
pub struct Snapshot {
    pub files: std::collections::HashMap<String, String> // numele fisierului -> hash-ul fisierului
}

#[derive(Clone, Debug)]
pub struct Commit {
    pub hash: String,
    pub branch_name: String,
    //hash urile commit ale parintilor(daca e merge,sau e un singur parinte pt un commit normal)
    pub parent: Option<Vec<String>>,
    pub message: String,
    pub snapshot: Snapshot,
    pub timestamp: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Status {
    pub add_files: Vec<String>,
    pub modifies_files: Vec<String>,
    pub removed_files: Vec<String>,
    pub staged_files: Vec<String>,
}
#[derive(Debug, Clone, Default, PartialEq)]
pub enum StatusFile {
    #[default]
    Same,
    AddFile,
    ModifiesFile,
    RemovedFile,
}
impl Status {
    pub fn new() -> Self {
        Self {
            add_files: Vec::new(),
            modifies_files: Vec::new(),
            removed_files: Vec::new(),
            staged_files: Vec::new(),
        }
    }
}
#[derive(Debug, Clone, Default)]
pub struct FileRepo {
    pub name: String, //calea catre fisier
    pub status: StatusFile,
    pub content: String,
    pub hash: String,
}

#[derive(Debug)]
pub struct Repository {
    pub current_branch: String,
    pub stage_area: Vec<FileRepo>,
}
