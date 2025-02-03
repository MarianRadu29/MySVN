use chrono::Utc;
use colored::*;
use ignore::gitignore::GitignoreBuilder;
use ignore::WalkBuilder;
use rusqlite::Connection;
use sha1::Sha1;
use sha2::{Digest, Sha256};
use std::fs::OpenOptions;
use std::io::{self, Read};
use std::{
    collections::HashMap,
    fs::{self, File},
    io::Write,
    path::Path,
};
use walkdir::WalkDir;

use crate::my_error::MyCostumError;
use crate::structures::*;

//in HEAD retin branch-ul curent
//in folderul branches retin pentru fiecare branch hash-ul ultimului commit

//(POSIBIL !!!!!) ar mai merge facut un json pentru stage_area,un caz ar fii: fac git add . si dupa inchid programul

//nume tabele:
//file_repo
//snapshot
//commitrepo
impl Repository {
    pub fn new() -> Result<Self, MyCostumError> {
        let current_branch;
        if Path::new(".svn/HEAD").exists() {
            current_branch = fs::read_to_string(String::from(".svn/HEAD"))?; //ar trebui facut un Result<Self,io::Error>
            return Ok(Self {
                current_branch,
                stage_area: Vec::new(),
            });
        }
        Ok(Self {
            current_branch: String::new(),
            stage_area: Vec::new(),
        })
    }
    pub fn init() -> Result<Self, MyCostumError> {
        if Path::new(".svn").exists() && Path::new(".svn").is_dir() {
            println!("This repository is already initialized!!"); //ar mai trebui testat daca sunt toate fisierele in acesta
            return Ok(Self {
                current_branch: fs::read_to_string(".svn/HEAD")?,
                stage_area: Vec::new(),
            });
        }
        fs::create_dir(".svn")?;
        write!(File::create(".svn/HEAD")?, "main")?;
        fs::create_dir(".svn/branches")?;
        File::create(get_parent_name()? + "/svn_ignore")?;
        File::create(".svn/branches/main")?; //deocamdata il las gol,cand fac primul commit treb sa l pun continutul lui hash-ul ultimului commit
        let conn = Connection::open(".svn/objects.db")?;
        let mut create = r#"
        create table file_repo (
            name TEXT,        
            content TEXT NOT NULL,
            branch TEXT NOT NULL,
            hash TEXT NOT NULL PRIMARY KEY
        );
    "#;
        conn.execute(create, ())?;

        // creez baza de date commitrepo
        create = r#"
        create table commitrepo (
            hash TEXT PRIMARY KEY,
            branch TEXT NOT NULL,
            parent TEXT,       
            message TEXT NOT NULL,
            timestamp TEXT NOT NULL
        );
    "#;
        conn.execute(create, [])?;

        // creez baza de date snapshot
        create = r#"
        create table snapshot (
            commit_hash TEXT NOT NULL,
            file_name TEXT NOT NULL,
            file_hash TEXT NOT NULL
        );
    "#;
        conn.execute(create, [])?;
        Ok(Self {
            current_branch: String::from("main"),
            stage_area: Vec::new(),
        })
    }
    pub fn is_init(&self) -> bool {
        Path::new(".svn").exists() && Path::new(".svn").is_dir()
    }
    pub fn get_current_branch(&self) -> String {
        self.current_branch.clone()
    }
    fn get_status(&self) -> Result<Status, MyCostumError> {
        let mut status = Status::new(); //ce trebuie returnat
        let path = format!(".svn/branches/{}", self.current_branch);
        let hash = fs::read_to_string(path)?;

        //testez daca branch-ul este gol
        if hash == String::new() {
            let parent = get_parent_name()?;
            let project_path = Path::new(parent.as_str());

            //calea catre fisierul ignore
            let svn_ignore_path = project_path.join("svn_ignore");

            // contruiesc regulile folosind GitignoreBuilder
            let mut builder = GitignoreBuilder::new(project_path);
            builder.add(svn_ignore_path);

            // contruiesc matcher-ul pentru a verifica regulile
            let matcher = builder.build()?;

            let walker = WalkBuilder::new(project_path)
                // aplic regulile de ignorare implicite
                .standard_filters(true)
                .build();

            let mut list_files = vec![];
            for result in walker {
                if result.is_err() {
                    continue;
                } //DEOCAMDATA las asa

                let entry = result?;
                let path = entry.path();
                if !matcher
                    .matched(path, entry.file_type().map_or(false, |ft| ft.is_dir()))
                    .is_ignore()
                {
                    let str = format!("{}", entry.path().display()).replace("\\\\", "/");

                    if str.is_empty() {
                        continue;
                    }
                    let ok = get_default_ignores()?.iter().any(|i| str.starts_with(i));
                    if ok {
                        continue;
                    }

                    if str == get_parent_name()? {
                        continue;
                    }
                    if fs::metadata(&str)?.len() < 1 {
                        continue;
                    }

                    //fisierul este in stage area
                    if self.stage_area.iter().any(|filerepo| filerepo.name == str) {
                        //daca fisierul este in stage area, testez daca nu l am modificat fata de momentul in care i am dat stage
                        //daca l am modificat pun la status modifies
                        //daca nu il adaug ca staged_file
                        let file = self.stage_area.iter().find(|x| x.name == str).unwrap(); //stim din if ul de mai sus ca exista
                        if Self::hash_file_sha1(&str)? == file.hash {
                            status.staged_files.push(str.clone());
                        } else {
                            status.staged_files.push(str.clone()); //fisierul este si in stage si in modifies
                            status.modifies_files.push(str.clone());
                        }
                        list_files.push(str.clone());
                        continue;
                    }
                    list_files.push(str.clone());
                    status.add_files.push(str.clone());
                }
            }

            if self.stage_area.is_empty() {
                //println!("{:?}", self.stage_area);
                for filerepo in self.stage_area.iter() {
                    if !list_files.iter().any(|x| *x == filerepo.name) {
                        status.removed_files.push(filerepo.name.clone());
                    }
                }
            }
        } else {
            //exista un last commit in branch
            //daca branch ul curent este gol inseamna ca am parinte un commit din alt branch
            let current_branch = if self.get_branch_commits(&self.current_branch)?.is_empty() {
                let mut result = String::new();
                let path = ".svn/branches".to_string();

                for entry in WalkDir::new(&path) {
                    let entry = entry?;

                    if entry.file_type().is_dir() {
                        continue; //ignor directoarele
                    }

                    let file_name;
                    if let Some(filename) = entry.file_name().to_str() {
                        file_name = filename;
                    } else {
                        return Err(MyCostumError::MyError(String::from("The conversion of a file to string failed, it does not comply with UTF-8 standards.")));
                    }

                    let branch = file_name.split('/').last().unwrap(); //stiu sigur ca am macar un element

                    if branch == self.current_branch {
                        continue; //ignor branch ul curent
                    }

                    let file_path = entry.path();
                    let content = fs::read_to_string(file_path).unwrap_or_else(|_| String::new());

                    if content == hash {
                        result = branch.to_string();
                        break;
                    }
                }
                result
            } else {
                self.current_branch.clone()
            };

            let last_snapshot = self.get_commit_by_hash(&current_branch, &hash)?.snapshot;

            let mut list_files: Vec<String> = Vec::new();
            let parent = get_parent_name()?;
            let project_path = Path::new(parent.as_str());

            //calea catre fisierul ignore
            let svn_ignore_path = project_path.join("svn_ignore");

            // contruiesc regulile folosind GitignoreBuilder
            let mut builder = GitignoreBuilder::new(project_path);
            builder.add(svn_ignore_path);

            // contruiesc matcher-ul pentru a verifica regulile
            let matcher = builder.build()?;

            let walker = WalkBuilder::new(project_path)
                // aplic regulile de ignorare implicite
                .standard_filters(true)
                .build();
            for result in walker {
                if result.is_err() {
                    continue;
                } //DEOCAMDATA las asa

                let entry = result?;
                let path = entry.path();
                if !matcher
                    //ignor directoarele
                    .matched(path, entry.file_type().map_or(false, |ft| ft.is_dir()))
                    .is_ignore()
                {
                    let str = format!("{}", entry.path().display()).replace("\\\\", "/");

                    if str.is_empty() {
                        continue;
                    }
                    let ok = get_default_ignores()?.iter().any(|i| str.starts_with(i));
                    if ok {
                        continue;
                    }

                    if str == get_parent_name()? {
                        continue;
                    }
                    if fs::metadata(&str)?.len() < 1 {
                        //fisierul este gol
                        continue;
                    }

                    list_files.push(str);
                }
            }
            //mai sus am luat lista cu fisierele ce sunt fizic in "repo"

            for (name_file, hash) in last_snapshot.files.iter() {
                //testez daca fisierul este in stage area
                if self
                    .stage_area
                    .iter()
                    .any(|filerepo| filerepo.name == *name_file)
                {
                    //daca fisierul este in stage area, testez daca nu l am modificat fata de momentul in care i am dat stage
                    //daca l am modificat pun la status modifies
                    //daca nu il adaug ca staged_file
                    let file = self
                        .stage_area
                        .iter()
                        .find(|x| x.name == *name_file)
                        .unwrap(); //stim din if ca fisierul este in stage area
                                   //println!("file: {}\nlast commit: {} staged: {}",name_file,hash,file.hash);
                    if *hash == file.hash {
                        status.staged_files.push(name_file.clone());
                    } else {
                        //testez daca fisierul din stage nu s-a modificat,
                        //daca s-a modificat pun ca e si in stage si ca e modificat
                        if file.hash == Self::hash_file_sha1(&file.name)? {
                            status.staged_files.push(name_file.clone());
                        } else {
                            status.staged_files.push(name_file.clone()); //fisierul este si in stage si in modifies
                            status.modifies_files.push(name_file.clone());
                        }
                    }
                    list_files.retain(|x| x != name_file);

                    continue;
                } //final testare stage area

                if list_files.contains(name_file) {
                    //fisierul exista in commit-ul anterior,o sa fie nemodificat sau modificat
                    //daca e nemodificat nu fac nimic
                    if Self::hash_file_sha1(name_file)? != *hash {
                        status.modifies_files.push(name_file.clone());
                    } //daca hash-urile sunt egale,nu adaug nimic la status
                    list_files.retain(|x| x != name_file);
                } else {
                    //fisierul a fost sters fata de ultimul commit
                    status.removed_files.push(name_file.clone());
                    list_files.retain(|x| x != name_file);
                }
            }

            //au ramas in list_files fisierele care nu erau in ultimul commit
            for file in list_files.iter() {
                //println!("{}",file);
                if !self
                    .stage_area
                    .iter()
                    .any(|filerepo| filerepo.name == *file)
                {
                    status.add_files.push(file.clone());
                } else {
                    status.staged_files.push(file.clone());
                }
            }
        }
        Ok(status)
    }

    pub fn print_status(&self) {
        let status = match self.get_status() {
            Ok(s) => s,
            Err(e) => {
                println!("Error at status command: {:?}", e);
                return;
            }
        };
        let nr = match get_parent_name() {
            Ok(n) => n.len() + 1,
            Err(e) => {
                println!("Filename formatting error: {:?}", e);
                return;
            }
        };

        println!("On branch {}\n", self.current_branch);
        if !status.staged_files.is_empty() {
            println!("Staged files: ");
            for file in &status.staged_files {
                let print_file = &file[nr..];
                println!("\t{}", print_file.green().bold());
            }
        }
        if !status.add_files.is_empty() {
            println!("Untracked files:");
            for file in &status.add_files {
                let print_file = &file[nr..];
                println!("\t{print_file}");
            }
        }
        if !status.modifies_files.is_empty() {
            println!("Modifies files:");
            for file in &status.modifies_files {
                let print_file = &file[nr..];
                println!("\t{}", print_file.yellow().bold());
            }
        }
        if !status.removed_files.is_empty() {
            println!("Removed files:");
            for file in &status.removed_files {
                let print_file = &file[nr..];
                println!("\t{}", print_file.red().bold());
            }
        }
        println!("\n");
    }

    fn hash_file_sha1(file_path: &str) -> io::Result<String> {
        let mut file = File::open(file_path)?;

        let mut buffer = Vec::new();
        let mut hasher = Sha1::new();

        file.read_to_end(&mut buffer)?;
        hasher.update(&buffer);

        let result = hasher.finalize();
        Ok(format!("{:x}", result))
    }

    fn generate_commit_hash(
        file_hashes: HashMap<String, String>,
        timestamp: &str,
        message: &str,
        branch_name: &str,
        parent_hash: Option<Vec<&str>>,
    ) -> String {
        // creez un hasher SHA256
        let mut hasher = Sha256::new();

        // adaug hash urile fisierelor
        for (_, file_hash) in file_hashes {
            hasher.update(file_hash.as_bytes());
        }

        // adaug timestamp-ul
        hasher.update(timestamp.as_bytes());

        // adaug mesajul commitului
        hasher.update(message.as_bytes());

        // adaug numele branch-ului
        hasher.update(branch_name.as_bytes());

        // adaug hash-ul commitului parinte, daca exista
        if let Some(parents) = parent_hash {
            for hash in parents.iter() {
                hasher.update(hash.as_bytes());
            }
        }

        // calculez hash-ul final
        let result = hasher.finalize();

        // convertesc hash-ul la un sir hexazecimal
        format!("{:x}", result)
    }

    pub fn get_branch_commits(&self, name_branch: &str) -> Result<Vec<Commit>, MyCostumError> {
        let conn = Connection::open(".svn/objects.db")?;
        let mut stmt = conn
            //ordonez crescator dupa timestamp,deci de la cel mai vechi la cel mai recent
            .prepare("select * from commitrepo where branch=? order by timestamp")?;
        let commit_iter = stmt.query_map([name_branch], |row| {
            let (hash, branch_name, parent, message, timestamp): (String, _, _, _, _) = (
                row.get(0)?,
                row.get(1)?,
                row.get::<_, Option<String>>(2)?.and_then(|s| {
                    if s.trim().is_empty() {
                        None
                    } else {
                        //apelez String::from pt toate elementele generate de whitespace
                        Some(s.split_whitespace().map(String::from).collect())
                    }
                }),
                row.get(3)?,
                row.get(4)?,
            );
            let snapshot = {
                let aux_hash = hash.clone();
                let mut stmt = conn
                    .prepare("SELECT file_name, file_hash FROM snapshot WHERE commit_hash = ?")?;
                let file_iter = stmt.query_map([aux_hash], |row| {
                    let file_name: String = row.get(0)?;
                    let file_hash: String = row.get(1)?;
                    Ok((file_name, file_hash))
                })?;

                let mut files = HashMap::new();
                for file in file_iter {
                    let (file_name, file_hash) = file?;
                    files.insert(file_name, file_hash);
                }
                Snapshot { files }
            };
            Ok(Commit {
                hash,
                branch_name,
                parent,
                message,
                snapshot,
                timestamp,
            })
        })?;
        let mut branch = Vec::new();
        for commit in commit_iter {
            let commit = commit?;
            branch.push(commit);
        }
        Ok(branch)
    }

    fn get_commit_by_hash(
        &self,
        name_branch: &str,
        hash_commit: &str,
    ) -> Result<Commit, MyCostumError> {
        let conn = Connection::open(".svn/objects.db")?;
        let mut stmt = conn.prepare("select * from commitrepo where branch=? and hash=?")?;
        let mut commit_iter = stmt.query_map([name_branch, hash_commit], |row| {
            let hash: String = row.get(0)?;
            let branch_name = row.get(1)?;
            let parent = row.get::<_, Option<String>>(2)?.and_then(|s| {
                if s.trim().is_empty() {
                    None
                } else {
                    Some(s.split_whitespace().map(String::from).collect())
                }
            });
            let message = row.get(3)?;
            let timestamp = row.get(4)?;
            let snapshot = {
                let aux_hash = hash.clone();
                let mut stmt = conn
                    .prepare("select file_name, file_hash from snapshot where commit_hash = ?")?;
                let file_iter = stmt.query_map([aux_hash], |row| {
                    let file_name: String = row.get(0)?;
                    let file_hash: String = row.get(1)?;
                    Ok((file_name, file_hash))
                })?;

                let mut files = HashMap::new();
                for file in file_iter {
                    let (file_name, file_hash) = file?;
                    files.insert(file_name, file_hash);
                }
                Snapshot { files }
            };
            let commit = Commit {
                hash,
                branch_name,
                parent,
                message,
                snapshot,
                timestamp,
            };
            //println!("{:?}",commit);
            Ok(commit)
        })?;

        if let Some(commit) = commit_iter.next() {
            Ok(commit?)
        } else {
            //daca interogarea de mai sus nu contine linii returnam o eroare
            Err(MyCostumError::DBError(rusqlite::Error::QueryReturnedNoRows))
        }
        //Ok(commit_iter.filter_map(Result::ok).collect::<Vec<Commit>>()[0].clone())
    }

    pub fn add(&mut self, s: &str) -> Result<(), MyCostumError> {
        let args = s.split_whitespace().collect::<Vec<&str>>();

        let last_commit_hash =
            fs::read_to_string(".svn/branches/".to_string() + self.current_branch.as_str())?;
        let current_status = self.get_status()?;
        if current_status == Status::new() {
            return Ok(());
        }
        if args[0] == "." {
            for file in &current_status.add_files {
                let file_repo = FileRepo {
                    status: StatusFile::AddFile,
                    name: file.clone(),
                    content: fs::read_to_string(file)?,
                    hash: Self::hash_file_sha1(file)?,
                };
                self.stage_area.push(file_repo);
            }
            for file in &current_status.modifies_files {
                if let Some(index) = self.stage_area.iter().position(|x| &x.name == file) {
                    //inlocuiesc starea fisierului din stage cu starea pe care am adaugat-o acm
                    self.stage_area.remove(index);
                }

                let file_repo = FileRepo {
                    status: StatusFile::ModifiesFile,
                    name: file.clone(),
                    content: fs::read_to_string(file)?,
                    hash: Self::hash_file_sha1(file)?,
                };
                self.stage_area.push(file_repo);
            }
            for file in &current_status.removed_files {
                if let Some(index) = self.stage_area.iter().position(|x| &x.name == file) {
                    //inlocuiesc starea fisierului din stage cu starea pe care am adaugat-o acm
                    self.stage_area.remove(index);
                }

                //cred ca puteam pune "" la hash
                let hash = self
                    .get_commit_by_hash(&self.current_branch, &last_commit_hash)?
                    .snapshot
                    .files
                    .get(file)
                    .unwrap() //daca e la status remove inseamna ca cheia sigur exista(adica fisierul este in ultimul commit)
                    .clone();
                let file_repo = FileRepo {
                    status: StatusFile::RemovedFile,
                    name: file.clone(),
                    content: String::new(),
                    hash,
                };
                self.stage_area.push(file_repo);
            }
            for file in &current_status.staged_files {
                let f = self.stage_area.iter().find(|x| x.name == *file).unwrap(); //stim sigur ca e in stage area(din for)
                let hash = Self::hash_file_sha1(file)?;
                if f.hash != hash {
                    self.stage_area.retain(|x| x.name == *file);
                    let file_repo = FileRepo {
                        status: StatusFile::ModifiesFile,
                        name: file.clone(),
                        content: fs::read_to_string(file)?,
                        hash,
                    };
                    self.stage_area.push(file_repo);
                }
            }
        } else {
            //de ex svn add ceva.txt
            let parent_name = get_parent_name()?;
            let files = args
                .iter()
                .map(|x| format!("{}\\{}", parent_name.as_str(), x))
                .collect::<Vec<String>>();
            for file in files {
                for add_file in current_status.add_files.iter() {
                    if add_file == &file {
                        let filerepo = FileRepo {
                            name: file.clone(),
                            status: StatusFile::AddFile,
                            content: fs::read_to_string(&file)?,
                            hash: Self::hash_file_sha1(&file)?,
                        };
                        self.stage_area.push(filerepo);
                    }
                }
                for modifies_file in current_status.modifies_files.iter() {
                    if modifies_file == &file {
                        if let Some(index) = self.stage_area.iter().position(|x| x.name == file) {
                            //inlocuiesc starea fisierului din stage cu starea pe care am adaugat-o acm
                            self.stage_area.remove(index);
                        }

                        let filerepo = FileRepo {
                            name: file.clone(),
                            status: StatusFile::ModifiesFile,
                            content: fs::read_to_string(&file)?,
                            hash: Self::hash_file_sha1(&file)?,
                        };
                        self.stage_area.push(filerepo);
                    }
                }
                for remove_file in current_status.removed_files.iter() {
                    if remove_file == &file {
                        if let Some(index) = self.stage_area.iter().position(|x| x.name == file) {
                            //inlocuiesc starea fisierului din stage cu starea pe care am adaugat-o acm
                            self.stage_area.remove(index);
                        }
                        let filerepo = FileRepo {
                            name: file.clone(),
                            status: StatusFile::RemovedFile,
                            content: String::new(),
                            hash: Self::hash_file_sha1(&file)?,
                        };
                        self.stage_area.push(filerepo);
                    }
                }
            }
        }
        Ok(())
        //preiau argumentele fisier sau foldere si le adaug in stage area
    }

    pub fn commit(&mut self, msg: &str) -> Result<(), MyCostumError> {
        if self.stage_area.is_empty() {
            println!(
                "You cannot commit, because you have no changes compared to the other commit!!"
            );
            return Ok(());
        }
        //println!("{}", self.current_branch);
        let path = format!(".svn/branches/{}", self.current_branch);
        let hash = fs::read_to_string(path)?;
        //daca n am parinte commit iau doar fisierele din stage area
        if hash.is_empty() {
            let mut commit = Commit {
                hash: String::new(),
                branch_name: self.current_branch.clone(),
                parent: None,
                message: msg.to_string(),
                snapshot: Snapshot {
                    files: {
                        let v = self
                            .stage_area
                            .iter()
                            .map(|x| (x.name.clone(), x.hash.clone()))
                            .collect::<Vec<(String, String)>>();
                        let mut result = HashMap::new();
                        for (name, hash) in v.iter() {
                            result.insert(name.clone(), hash.clone());
                        }
                        result
                    },
                },
                timestamp: format!("{}", Utc::now().timestamp()),
            };
            commit.hash = Self::generate_commit_hash(
                commit.snapshot.files.clone(),
                &commit.timestamp,
                msg,
                &self.current_branch,
                None,
            );

            let mut insert_query = r#"
                insert into commitrepo (hash, branch, parent, message, timestamp)
                values (?1, ?2, ?3, ?4, ?5);
            "#;
            let conn = Connection::open(".svn/objects.db")?;

            let parent = commit.parent.unwrap_or_default(); //ori imi returneaza parintele ori ""
            conn.execute(
                insert_query,
                [
                    &commit.hash,
                    &commit.branch_name,
                    parent.join(" ").trim(),
                    &commit.message,
                    &commit.timestamp,
                ],
            )?;

            insert_query = r#"
                insert into snapshot (commit_hash, file_name,file_hash)
                values(?1,?2,?3);
            "#;
            for filerepo in self.stage_area.iter() {
                conn.execute(insert_query, [&commit.hash, &filerepo.name, &filerepo.hash])?;
            }

            insert_query = r#"
                insert into file_repo (name,content,hash,branch)
                values(?1,?2,?3,?4);
            "#;
            for filerepo in self.stage_area.iter() {
                conn.execute(
                    insert_query,
                    [
                        &filerepo.name,
                        &filerepo.content,
                        &filerepo.hash,
                        &self.current_branch,
                    ],
                )?;
            }
            let path = format!(".svn/branches/{}", self.current_branch);
            let mut file = File::create(path)?;
            file.write_all(commit.hash.as_bytes())?;
            self.stage_area.clear();
        } else {
            //daca am parinte ,preiau fisierele din acel commit si modific cu ce este in stage area
            let mut commit = Commit {
                hash: String::new(),
                branch_name: self.current_branch.clone(),
                parent: Some(hash.split_whitespace().map(String::from).collect()),
                message: msg.to_string(),
                snapshot: Snapshot {
                    files: {
                        let conn = Connection::open(".svn/objects.db")?;
                        let mut stmt = conn.prepare(
                            "SELECT file_name, file_hash FROM snapshot WHERE commit_hash = ?",
                        )?;
                        let file_iter = stmt.query_map([&hash], |row| {
                            let file_name: String = row.get(0)?;
                            let file_hash: String = row.get(1)?;
                            Ok((file_name, file_hash))
                        })?;

                        let mut files = HashMap::new();
                        for file in file_iter {
                            let (file_name, file_hash) = file?;
                            files.insert(file_name, file_hash);
                        }

                        files
                    },
                },
                timestamp: format!("{}", Utc::now().timestamp()),
            };
            for file_stage in self.stage_area.iter() {
                match file_stage.status {
                    StatusFile::AddFile | StatusFile::ModifiesFile => {
                        commit
                            .snapshot
                            .files
                            .insert(file_stage.name.clone(), file_stage.hash.clone());
                    }
                    StatusFile::RemovedFile => {
                        commit.snapshot.files.remove(&file_stage.name.clone());
                    }
                    _ => {}
                }
            }

            commit.hash = Self::generate_commit_hash(
                commit.snapshot.files.clone(),
                &commit.timestamp,
                msg,
                &self.current_branch,
                None,
            );

            let mut insert_query = r#"
                insert into commitrepo (hash, branch, parent, message, timestamp)
                values (?1, ?2, ?3, ?4, ?5);
            "#;
            let conn = Connection::open(".svn/objects.db")?;
            let parent = commit.parent.unwrap_or_default();
            conn.execute(
                insert_query,
                [
                    &commit.hash,
                    &commit.branch_name,
                    parent.join(" ").trim(),
                    &commit.message,
                    &commit.timestamp,
                ],
            )?;

            insert_query = r#"
                insert into snapshot (commit_hash, file_name,file_hash)
                values(?1,?2,?3);
            "#;
            for filerepo in commit.snapshot.files.iter() {
                conn.execute(
                    insert_query,
                    [commit.hash.clone(), filerepo.0.clone(), filerepo.1.clone()],
                )?;
            }
            insert_query = r#"
                insert into file_repo (name,content,hash,branch)
                values(?1,?2,?3,?4);
            "#;
            for filerepo in self.stage_area.iter() {
                if filerepo.status == StatusFile::AddFile
                    || filerepo.status == StatusFile::ModifiesFile
                {
                    //println!("{}",filerepo.name);
                    conn.execute(
                        insert_query,
                        [
                            &filerepo.name,
                            &filerepo.content,
                            &filerepo.hash,
                            &self.current_branch,
                        ],
                    )?;
                }
            }

            let path = format!(".svn/branches/{}", self.current_branch);
            let mut file = File::create(path)?;
            file.write_all(commit.hash.as_bytes())?;
            self.stage_area.clear();
        }
        Ok(())
    }

    pub fn create_branch(&self, branch: &str) -> Result<(), MyCostumError> {
        let mut ok = false;
        let base_path = ".svn/branches/";
        let new_branch = base_path.to_string() + branch;

        for entry in fs::read_dir(base_path)? {
            let entry = entry?;
            let path = format!("{}", entry.path().display());
            //println!("{} <=> {}",new_branch,path);
            if path == new_branch {
                ok = true;
                break;
            }
        }
        if ok {
            println!("This branch already exists");
            return Ok(());
        }

        File::create(new_branch)?;
        Ok(())
    }

    pub fn create_branch_with_parent(&self, name_branch: &str) -> Result<(), MyCostumError> {
        let current_branch = fs::read_to_string(".svn/HEAD")?;
        let last_commit_hash = fs::read_to_string(format!(".svn/branches/{}", current_branch))?;
        if last_commit_hash.is_empty() {
            println!("The source branch has no commits, I cannot create a branch with a common commit with it");
            return Ok(());
        }

        let merge_base_hash =
            Self::get_commit_by_hash(self, &current_branch, &last_commit_hash)?.hash;
        let path = format!(".svn/branches/{}", name_branch);
        let mut file = File::create(path)?;
        file.write_all(merge_base_hash.as_bytes())?;
        Ok(())
    }

    pub fn diff_between_branches(&self, branch1: &str, branch2: &str) -> Result<(), MyCostumError> {
        if !self.exists_branch(branch1)? {
            println!("The branch {} doesn't exist", branch1);
            return Ok(());
        }
        if !self.exists_branch(branch2)? {
            println!("The branch {} doesn't exist", branch2);
            return Ok(());
        }

        let branch1_commits = self.get_branch_commits(branch1)?;
        let branch2_commits = self.get_branch_commits(branch2)?;
        if branch1_commits.is_empty() && branch2_commits.is_empty() {
            println!("Both branches are empty. No differences to display.");
            return Ok(());
        }
        if branch1_commits.is_empty() || branch2_commits.is_empty() {
            let output = if branch1_commits.is_empty() {
                branch1
            } else {
                branch2
            };
            println!("No differences. ({} branch is empty.)", output);
            return Ok(());
        }

        //stim de mai sus ca branch urile nu sunt goale,deci exista un "last"
        let mut branch1_snapshot = branch1_commits.last().unwrap().snapshot.clone();
        let mut branch2_snapshot = branch2_commits.last().unwrap().snapshot.clone();

        let mut list_file_both_contains = Vec::new();

        for (file, hash) in &branch1_snapshot.files {
            if branch2_snapshot.files.contains_key(file) {
                if hash != branch2_snapshot.files.get(file).unwrap() {
                    //preiau din baza de date continutul fisierelor si le fac diff
                    let conn = Connection::open(".svn/objects.db")?;
                    let mut stmt = conn.prepare(
                        r#"
                        select content from file_repo where hash=? and branch=?;
                        "#,
                    )?;
                    let file1_content = stmt
                        .query_map([hash, branch1], |row| row.get::<_, String>(0))?
                        .map(Result::unwrap)
                        .collect::<Vec<String>>()[0]
                        .clone(); //stim ca interogarea are o singura linie
                    let file2_content = stmt
                        .query_map(
                            [branch2_snapshot.files.get(file).unwrap(), branch2],
                            |row| Ok(row.get::<_, String>(0).unwrap()),
                        )?
                        .map(Result::unwrap)
                        .collect::<Vec<String>>()[0]
                        .clone();
                    let file_print = &file[get_parent_name()?.len() + 1..];
                    println!("File: {}\n", file_print);
                    for diff in diff::lines(&file1_content, &file2_content) {
                        match diff {
                            diff::Result::Left(l) => println!("{}", format!("-{}", l).red().bold()),
                            diff::Result::Both(l, _) => println!(" {}", l),
                            diff::Result::Right(r) => {
                                println!("{}", format!("+{}", r).green().bold())
                            }
                        }
                    }
                    println!("\n\n");
                }
                list_file_both_contains.push(file.clone())
            }
        }

        for file in &list_file_both_contains {
            branch2_snapshot.files.remove(file);
            branch1_snapshot.files.remove(file);
        }

        //in snapshot uri au ramas fisierele care nu sunt in comun intre cele 2 brranch uri
        let nr = get_parent_name()?.len() + 1;
        if !branch1_snapshot.files.is_empty() {
            println!("Files remain in {}", branch1);
            for (name, _) in branch1_snapshot.files {
                println!("\t{}", &name[nr..].green().bold());
            }
        }

        if !branch2_snapshot.files.is_empty() {
            println!("Files remain in {}", branch2);
            for (name, _) in branch2_snapshot.files {
                println!("\t{}", &name[nr..].green().bold());
            }
        }
        Ok(())
    }

    pub fn diff_with_last_commit(&self) -> Result<(), MyCostumError> {
        let current_branch_commits = self.get_branch_commits(&self.current_branch)?;
        let n = current_branch_commits.len();
        if n < 1 {
            println!("The current branch doesn't contain a commit");
            return Ok(());
        }

        let mut list_files: Vec<FileRepo> = Vec::new();

        let last_commit_snapshot = &current_branch_commits[n - 1].snapshot;
        for file in &self.stage_area {
            //daca fisierul este modificat atunci se face diff
            if file.status == StatusFile::ModifiesFile {
                //fac diff-ul efectiv intre cele 2 fisiere
                //cu plus fac la stagea area!!
                let conn = Connection::open(".svn/objects.db")?;
                let mut stmt = conn.prepare(
                    r#"
                        select content from file_repo where hash=? and branch=?;
                        "#,
                )?;
                let file_last_commit_content = stmt
                    .query_map(
                        [
                            last_commit_snapshot.files.get(&file.name).unwrap(),
                            &self.current_branch,
                        ],
                        |row| Ok(row.get::<_, String>(0).unwrap()),
                    )?
                    .map(Result::unwrap)
                    .collect::<Vec<String>>()[0]
                    .clone(); //stim ca interogarea are o singura linie
                let nr = get_parent_name()?.len() + 1;
                println!("File: {}\n", &file.name[nr..]);
                for diff in diff::lines(&file_last_commit_content, &fs::read_to_string(&file.name)?)
                {
                    match diff {
                        diff::Result::Left(l) => println!("{}", format!("-{}", l).red().bold()),
                        diff::Result::Both(l, _) => println!(" {}", l),
                        diff::Result::Right(r) => println!("{}", format!("+{}", r).green().bold()),
                    }
                }
                println!("\n\n");
            } else {
                //fisierul este nou sau sters fata de ultimul commit
                list_files.push(file.clone());
            }
        }
        if list_files.is_empty() {
            return Ok(());
        }
        if list_files.is_empty() {
            return Ok(());
        }
        println!("The remaining stage files status:\n");
        let (add_files, removed_files): (Vec<_>, Vec<_>) = list_files
            .into_iter()
            .partition(|x| x.status == StatusFile::AddFile);

        let nr = get_parent_name()?.len() + 1;

        if !add_files.is_empty() {
            println!("Add files:");
            for file in add_files {
                println!("\t{}", &file.name[nr..].green().bold());
            }
            println!();
        }

        if !removed_files.is_empty() {
            println!("Removed files:");
            for file in removed_files {
                println!("\t{}", &file.name[nr..].red().bold());
            }
            println!();
        }

        Ok(())
    }

    fn exists_branch(&self, branch: &str) -> Result<bool, MyCostumError> {
        let base_path = ".svn/branches/";
        let new_branch = base_path.to_string() + branch;

        for entry in fs::read_dir(base_path)? {
            let entry = entry?;
            let path = format!("{}", entry.path().display());
            //println!("{} <=> {}",new_branch,path);
            if path == new_branch {
                return Ok(true);
            }
        }
        Ok(false)
    }

    pub fn switch_branch(&mut self, branch: &str) -> Result<(), MyCostumError> {
        if !self.exists_branch(branch)? {
            println!("The branch doesn't exist");
            return Ok(());
        }

        let mut file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .open(".svn/HEAD")?;
        write!(file, "{}", branch)?;
        self.current_branch = branch.to_string();
        Ok(())
    }

    pub fn merge(&mut self, target_branch: &str) -> Result<(), MyCostumError> {
        let target_branch_path = format!(".svn/branches/{}", target_branch);
        if !Path::new(&target_branch_path).exists() {
            println!("The {} branch doesn't exist.", target_branch);
            return Ok(());
        }

        let current_branch_path = format!(".svn/branches/{}", self.current_branch);
        let current_commit_hash = fs::read_to_string(current_branch_path)?;

        let target_commit_hash = fs::read_to_string(target_branch_path)?;

        if current_commit_hash == target_commit_hash {
            println!("Branches are already synchronized.");
            return Ok(());
        }

        let merge_base = self.find_merge_base(&self.current_branch, target_branch)?;

        if merge_base.is_none() {
            println!("Branches don't have a common history.");
            return Ok(());
        }

        let current_snapshot = self
            .get_commit_by_hash(&self.current_branch, &current_commit_hash)?
            .snapshot;
        let target_snapshot = self
            .get_commit_by_hash(target_branch, &target_commit_hash)?
            .snapshot;

        // snapshot in care combin cele 2 branch uri
        let mut merged_snapshot = current_snapshot.clone();

        for (file, target_hash) in target_snapshot.files {
            if let Some(current_hash) = current_snapshot.files.get(&file) {
                if current_hash != &target_hash {
                    //prefer versiunea branchului target
                    let nr = get_parent_name()?.len() + 1;
                    println!(
                        "Conflict in file {}. We are using the version in {}.",
                        &file[nr..],
                        target_branch
                    );
                }
            }
            merged_snapshot.files.insert(file, target_hash);
        }

        let parents = vec![current_commit_hash, target_commit_hash];
        // creez un nou commit rezultat al merge-ului
        let mut merge_commit = Commit {
            hash: String::new(),
            branch_name: self.current_branch.clone(),
            parent: Some(parents.clone()),
            message: format!("Merge from {} in {}", target_branch, self.current_branch),
            snapshot: merged_snapshot,
            timestamp: format!("{}", Utc::now().timestamp()),
        };

        let merge_commit_hash = Self::generate_commit_hash(
            merge_commit.snapshot.files.clone(),
            &merge_commit.timestamp,
            &merge_commit.message,
            &self.current_branch,
            Some(parents.clone().iter().map(|x| x.as_str()).collect()),
        );
        merge_commit.hash = merge_commit_hash;

        let conn = Connection::open(".svn/objects.db")?;
        let insert_query = r#"
            insert into commitrepo (hash, branch, parent, message, timestamp)
            values (?1, ?2, ?3, ?4, ?5);
        "#;
        conn.execute(
            insert_query,
            [
                &merge_commit.hash,
                &merge_commit.branch_name,
                &merge_commit
                    .parent
                    .unwrap_or_default()
                    .join(" ")
                    .trim()
                    .to_string(),
                &merge_commit.message,
                &merge_commit.timestamp,
            ],
        )?;

        let insert_snapshot = r#"
            insert into snapshot (commit_hash, file_name, file_hash)
            values (?1, ?2, ?3);
        "#;
        for (file_name, file_hash) in merge_commit.snapshot.files {
            conn.execute(
                insert_snapshot,
                [&merge_commit.hash, &file_name, &file_hash],
            )?;
        }

        //nu trebuie sa mai introduc fisiere,ele deja exista in baza de date

        let branch_path = format!(".svn/branches/{}", merge_commit.branch_name);
        let mut file = File::create(branch_path)?;
        file.write_all(merge_commit.hash.as_bytes())?;
        Ok(())
    }

    //caut commit ul comun dintre cele 2  branch uri
    fn find_merge_base(
        &self,
        source_branch: &str,
        target_branch: &str,
    ) -> Result<Option<String>, MyCostumError> {
        let commits_source_branch = self.get_branch_commits(source_branch)?;
        let first_commit_target_hash_parent = &self.get_branch_commits(target_branch)?;

        let hash: Vec<String>;
        if let Some(first_commit) = first_commit_target_hash_parent.first() {
            if let Some(n) = &first_commit.parent {
                hash = n.clone();
            } else {
                return Ok(None);
            }
        } else {
            return Ok(None);
        }

        for commit1 in commits_source_branch.iter() {
            if commit1.hash == hash[0] {
                return Ok(Some(commit1.hash.clone()));
            }
        }

        Ok(None)
    }

    pub fn restore_stage_area(&mut self) {
        self.stage_area.clear();
    }
}
