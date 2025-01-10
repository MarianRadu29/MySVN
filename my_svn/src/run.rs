use crate::structures::*;
use chrono::{Local, Utc};
use colored::Colorize;
use shell_words::split; //pentru a parsa un string intr un vector de args
use std::io::{self,Write};

#[allow(non_upper_case_globals)]
const command_not_found: &str = "Command is not found!!";

#[allow(dead_code)] //cand rulez pe feature-ul restore o sa dea warning de unused
pub fn run() {
    let mut input;
    let mut repo = match Repository::new() {
        Ok(r) => r,
        Err(e) => panic!("Error initialization repository: {e}"),
    };
    loop {
        print!("> ");
        io::stdout().flush().expect("Error unblocking stdout");
        input = String::new();
        io::stdin()
            .read_line(&mut input)
            .expect("Error reading from stdin");
        input = input.trim().to_string();
        if input == String::new() {
            continue;
        }
        let args = match split(&input) {
            Ok(v) => v,
            Err(e) => {
                println!("{}", format!("Error parse arguments: {e:?}").red().bold());
                continue;
            }
        };
        println!();
        if args.len() == 1 && args[0] == "svn" {
            //comanda pt afisare detalii comenzii
            println!("Available {} commands:\n","svn".to_string().bold().yellow());
            println!("\texit\t\t- Exit the application.");
            println!("\tinit\t\t- Initializes the repository.");
            println!("\tnr\t\t- Displays the number of commits in the current branch.");
            println!("\tstatus\t\t- Displays the repository status.");
            println!("\tbranch\t\t- Displays the current branch.");
            println!("\tdiff\t\t- Shows the diff of staged changes.");
            println!("\treset\t\t- Restores the staging area.");
            println!("\tlog\t\t- Print all commits from current branch.");
            println!("\tdiff <branch1> <branch2> - Diff between two branches.");
            println!("\tadd <files>\t- Adds files to the staging area.");
            println!("\tcommit <msg>\t- Commits changes with a message.");
            println!("\tswitch <branch>\t- Switches to another branch.");
            println!("\tmerge <branch>\t- Merges the specified branch.");
            println!("\tbranch <branch>\t- Creates a branch.");
            println!("\tbranch -p <branch> - Create a branch with the parent last commit from the current branch.");
            continue;
        }
        if args[0] != "svn" {
            println!("{}", command_not_found.red().bold());
            continue;
        }
        if args.len() == 2 {
            match args[1].as_str() {
                "exit" => break,
                "init" => match Repository::init() {
                    Ok(init) => repo = init,
                    Err(e) => println!(
                        "{}",
                        format!("Initialization error:\n\t{:?}", e).red().bold()
                    ),
                },
                "nr" => {
                    if !repo.is_init() {
                        println!(
                            "{}",
                            "The repository is not initialized.".to_string().bold().red()
                        );
                        continue;
                    }
                    match repo.get_branch_commits(&repo.get_current_branch()) {
                        Ok(commits) => println!("{}", commits.len()),
                        Err(e) => println!(
                            "{}",
                            format!("Error retrieving branch commit number:\n\t{:?}", e)
                                .red()
                                .bold()
                        ),
                    };
                }
                "status" => {
                    if !repo.is_init() {
                        println!(
                            "{}",
                            "The repository is not initialized.".to_string().bold().red()
                        );
                        continue;
                    }
                    repo.print_status()
                }
                "branch" => {
                    if !repo.is_init() {
                        println!(
                            "{}",
                            "The repository is not initialized.".to_string().bold().red()
                        );
                        continue;
                    }
                    println!("{}", repo.get_current_branch())
                }
                "diff" => {
                    if !repo.is_init() {
                        println!(
                            "{}",
                            "The repository is not initialized.".to_string().bold().red()
                        );
                        continue;
                    }
                    if let Err(e) = repo.diff_with_last_commit() {
                        println!("{}", format!("Error diff staged:\n\t{e:?}").red().bold())
                    }
                }
                "log" => {
                    if !repo.is_init() {
                        println!(
                            "{}",
                            "The repository is not initialized.".to_string().bold().red()
                        );
                        continue;
                    }
                    match repo.get_branch_commits(&repo.get_current_branch()) {
                        Ok(commits) => {
                            for commit in commits.iter().rev() {
                                println!(
                                    "{} {}",
                                    "commit".yellow().bold(),
                                    commit.hash.bold().yellow()
                                );

                                let datetime = chrono::DateTime::<Utc>::from_timestamp(
                                    commit.timestamp.parse::<i64>().unwrap(), //o sa mearga deoarece timestamp este un numar i64 facut string
                                    0,
                                )
                                .unwrap(); //timestampul meereu o sa fie corect(returneaza eroare daca timestampul este invalid,dar eu l am generat corect)

                                //a=day of the week , b=month of the year d=day of the month
                                println!(
                                    "Date:\t{}",
                                    datetime
                                        .with_timezone(&Local)
                                        .format("%a %b %d %H:%M:%S %Y")
                                );
                                println!("\n\t{}\n", commit.message);
                            }
                        }
                        Err(e) => println!(
                            "{}",
                            format!("Error retrieving branch logs:\n\t{:?}", e)
                                .red()
                                .bold()
                        ),
                    }
                }
                "reset" => {
                    if !repo.is_init() {
                        println!(
                            "{}",
                            "The repository is not initialized.".to_string().bold().red()
                        );
                        continue;
                    }
                    repo.restore_stage_area();
                }
                _ => println!("{}", command_not_found.red().bold()),
            };
            continue;
        }

        if args[1] == "diff" {
            if !repo.is_init() {
                println!(
                    "{}",
                    "The repository is not initialized.".to_string().bold().red()
                );
                continue;
            }
            if args.len()<4{
                println!("{}","Invalid command! Possible command format: svn diff <branch1> <branch2>".to_string().bold().red());
                continue;
            }
            if let Err(e) = repo.diff_between_branches(&args[2], &args[3]) {
                println!(
                    "{}",
                    format!("Error at diff between branches:\n\t{e:?}")
                        .red()
                        .bold()
                );
            }
            continue;
        }
        if args[1] == "add" {
            if !repo.is_init() {
                println!(
                    "{}",
                    "The repository is not initialized.".to_string().bold().red()
                );
                continue;
            }
            let str = args[2..].join(" ");
            if let Err(e) = repo.add(&str) {
                println!("{}", format!("Error at add command:\n\t{e:?}").red().bold());
            }
            continue;
        }
        if args[1] == "commit" {
            if !repo.is_init() {
                println!(
                    "{}",
                    "The repository is not initialized.".to_string().bold().red()
                );
                continue;
            }
            if let Err(e) = repo.commit(&args[2]) {
                println!(
                    "{}",
                    format!("Error at commit command:\n\t{e:?}").red().bold()
                );
            }
            continue;
        }
        if args[1] == "switch" {
            if !repo.is_init() {
                println!(
                    "{}",
                    "The repository is not initialized.".to_string().bold().red()
                );
                continue;
            }
            if let Err(e) = repo.switch_branch(&args[2]) {
                println!(
                    "{}",
                    format!("Error at switch command:\n\t{e:?}").red().bold()
                );
            }
            continue;
        }
        if args[1] == "branch" {
            if !repo.is_init() {
                println!(
                    "{}",
                    "The repository is not initialized.".to_string().bold().red()
                );
                continue;
            }
            if args.len() == 4 && args[2] == "-p" {
                if let Err(e) = repo.create_branch_with_parent(&args[3]) {
                    println!(
                        "{}",
                        format!("Error at create branch command:\n\t{e:?}")
                            .red()
                            .bold()
                    )
                }
                continue;
            }
            if args.len() == 3 {
                if !repo.is_init() {
                    println!(
                        "{}",
                        "The repository is not initialized.".to_string().bold().red()
                    );
                    continue;
                }
                if let Err(e) = repo.create_branch(&args[2]) {
                    println!(
                        "{}",
                        format!("Error at create branch command:\n\t{e:?}")
                            .red()
                            .bold()
                    )
                }
                continue;
            }
        }
        if args[1] == "merge" {
            if !repo.is_init() {
                println!(
                    "{}",
                    "The repository is not initialized.".to_string().bold().red()
                );
                continue;
            }
            if let Err(e) = repo.merge(&args[2]) {
                println!("{}", format!("Err at merge command:\n\t{e:?}").red().bold());
            }
            continue;
        }
        println!("{}", command_not_found.red().bold());
    }
}
