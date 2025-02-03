# MY SVN

MySVN is a local version control system designed to track changes in files and directories on a single machine. Unlike traditional SVN, it does not support remote repositories or collaboration but provides essential functionalities for managing revisions, restoring previous versions, and keeping a structured history of modifications. It is useful for individual developers who need a simple and efficient way to handle versioning without relying on external servers.


## Features

 - **Multiple Branch**: It is capable of maintaining multiple branches just like `git`;
 - **Checkout between branches**: Allows movement on multiple branches;
 - **Commits separate on each branches** (just like `git`);
 - **Simple merges**: Allows merging two branches, provided they have a commit in common;
 - **Diff between branches and with the previous commit**;
 - **Status functionality**(just like `git`);
 - **.gitignore functionality**;


 ## How to Run the Application

 ### Step 1: Clone the Repository

Clone the repository to your local system:
```bash
git clone https://github.com/MarianRadu29/MySVN
cd MySVN/my_svn
```
---

### Step 2: Install [**Cargo**](https://win.rustup.rs/) tool & [**VSCode**](https://code.visualstudio.com/download) with all the extensions below:
      - rust-analyzer (id: rust-lang.rust-analyzer)
      - Even Better TOML (id: tamasfe.even-better-toml)
      - CodeLLDB (id: vadimcn.vscode-lldb)
    
---
    
### Step 3: Open the terminal and run this command: 
```bash
cargo run
```

---

## List run commands:
 - `cargo run` - comportament normal;
 - `cargo test -- --nocapture` - displays detailed information about each commit stored in the **SVN** object on the screen;
 - `cargo run --no-default-capture --features restore` - restores the entire git object, deletes all commits and all branches except main (it's like opening my_svn for the first time and typing the `svn init` command);

## If you want to know all the commands and what they do, type `svn`