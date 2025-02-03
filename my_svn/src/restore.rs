//#[allow(dead_code)] //cand schimb feature-ul o sa mi apeleze functia asta
#[cfg(feature = "restore")]
pub fn restore_svn() -> Result<(), MyCostumError> {
    use crate::my_error::MyCostumError;
    use crate::structures::get_parent_name;
    use std::{fs::OpenOptions, io::Write};


    let mut file = OpenOptions::new()
        .write(true)
        .open(get_parent_name()? + "/svn_ignore")?;

    file.set_len(0)?;
    file = OpenOptions::new().write(true).open(".svn/HEAD")?;
    file.set_len(0)?;
    file.write_all("main".as_bytes())?;

    //println!("daaa");
    let dir = ".svn/branches";
    for entry in walkdir::WalkDir::new(dir) {
        match entry {
            Ok(entry) => {
                if entry.path().is_dir() {
                    continue;
                }
                if let Some(file_name) = entry.file_name().to_str() {
                    if file_name == "main" {
                        OpenOptions::new()
                            .write(true)
                            .open(entry.path())?
                            .set_len(0)?;
                    } else {
                        std::fs::remove_file(entry.path())?;
                    }
                }
            }
            Err(e) => eprintln!("Error while parsing: {}", e),
        }
    }

    file = OpenOptions::new()
        .truncate(true)
        .write(true)
        .open(".svn/HEAD")?;
    file.write_all("main".as_bytes())?;

    let conn = rusqlite::Connection::open(".svn/objects.db")?;

    conn.execute("DELETE FROM file_repo", [])?;
    conn.execute("DELETE FROM commitrepo", [])?;
    conn.execute("DELETE FROM snapshot", [])?;

    println!("\nThe repository has been restore.");
    Ok(())
}
