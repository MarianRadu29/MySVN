#[cfg(test)]
mod tests {
    use crate::structures::{get_parent_name, Commit, Snapshot};
    use rusqlite::Connection;
    use std::collections::HashMap;

    fn test() -> Result<(), rusqlite::Error> {
        let conn = Connection::open(".svn/objects.db")?;

        let mut stmt =
            conn.prepare("select hash, branch, parent, message, timestamp from commitrepo")?;

        let commit_iter = stmt.query_map([], |row| {
            Ok(Commit {
                hash: row.get(0)?,
                branch_name: row.get(1)?,
                parent: row.get::<_, Option<String>>(2).unwrap().and_then(|s| {
                    if s.trim().is_empty() {
                        None
                    } else {
                        Some(s.split_whitespace().map(|x| String::from(x)).collect())
                    }
                }),
                message: row.get(3)?,
                timestamp: row.get(4)?,
                snapshot: Snapshot {
                    files: HashMap::new(),
                },
            })
        })?;

        for commit in commit_iter {
            let commit = commit?;
            println!("\n{:?}\n", commit);
        }
        println!();
        if let Ok(parent) = get_parent_name() {
            println!("Directory project: {parent}\n");
        }
        Ok(())
    }

    #[test]
    fn run() {
        if let Err(e) = test() {
            println!("{e}");
        }
    }
}
