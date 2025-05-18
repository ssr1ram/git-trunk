# git trunk

git trunk is a CLI tool for managing repository-wide documentation in a .trunk directory within a Git repository, stored under refs/trunk/main in the main repository.

Use it to hold information such as issues, bugs, changelog etc. These files can be maintained repowide and not be branch specific. 

## Typical flow

### Using it in a repo for the first time
```sh
❯ git trunk init          
🐘 ✓ Step 1: Confirmed inside a Git repository
🐘 ✓ Step 2: Repository root found at <dirpath>>
🐘 ✓ Step 3: Added .trunk to .gitignore
🐘 ✓ Step 4: .trunk directory created
🐘 ✓ Step 5: Created .trunk/readme.md
🐘 ✓ Step 6: Git repository initialized
🐘 ✓ Step 7: Files staged
🐘 ✓ Step 8: Initial commit created
🐘 ✅ Trunk initialized successfully
```
This 
- creates a new .trunk directiry that is .gitignore'd.
- allows you toadd any files you choose in .trunk which are later added to the .git database
- A new refs/trunk/main is created in the .git db

### Using it in a newly cloned repo that has a trunk (refs/trunk/main)
```sh
❯ git trunk checkout
🐘 ✓ Step 1: Repository root found at /Users/subadhrasriram/Documents/lucky/git-trunk-test/e11
🐘 ✓ Step 2: refs/trunk/main not found locally
🐘 ✓ Step 3: refs/trunk/main found on remote (origin)
🐘 ✓ Step 4: Successfully fetched refs/trunk/main
🐘 ✓ Step 5: refs/trunk/main verified locally
🐘 Step 6: .trunk directory does not exist
🐘 ✓ Step 7: .trunk directory created
🐘 ✓ Step 8: Git repository initialized in .trunk
🐘 ✓ Step 9: Successfully fetched refs/trunk/main into temporary ref
🐘 ✓ Step 10: Fetched commit hash: 9d9d7fc92c440553709a9e763a43d59ad4e2ee47
🐘 ✓ Step 11: Main branch reset to commit 9d9d7fc92c440553709a9e763a43d59ad4e2ee47
🐘 ✓ Step 12: refs/heads/main updated
🐘 ✓ Step 13: Temporary ref cleaned up
🐘 ✅ Trunk checkout successfully
```

This
- fetches refs/trunk/main from origin 
- initialize the .trunk directory and repopulates it with the files

### Everyday usage
- Make changes to .trunk files and then run `git trunk commit` and `git trunk push`
- You can use `git trunk hooks` to setup 
  - a post-commit hook that does the `git trunk commit` on every regular commit
  - a pre-push hook that does the `git trunk push` on every git push 

```sh
❯ git trunk commit
🐘 ✓ Step 1: Repository root found at <dirpath>
🐘 ✓ Step 2: .trunk directory found
🐘 ≠ Step 4: Changes detected in .trunk:
 M readme.md

🐘︖ Stage all files? [y/N]: y
🐘 ✓ Step 4: Files staged
🐘 ✓ Step 5: Changes committed
🐘 ✓ Step 7: Objects fetched
🐘 ✓ Step 8: Updated refs/trunk/main to commit e9278...
🐘 ✅ Trunk commited successfully
```

```sh
❯ git trunk push
🐘 ✓ Step 1: refs/trunk/main found locally
🐘 ✓ Step 2: Successfully pushed refs/trunk/main to origin
🐘 ✅ Trunk pushed successfully
```

## git-trunk help
```
❯ git trunk
Git Trunk CLI for managing repository-wide documents

Usage: git-trunk [OPTIONS] <COMMAND>

Commands:
  init      Initializes the git-trunk in the current repository
  commit    Commits changes from .trunk to the main repository
  checkout  Checkouts the trunk from refs/trunk/main into .trunk
  push      Pushes the objects from refs/trunk/main to remote (default origin)
  hooks     Manages Git hooks for git-trunk
  help      Print this message or the help of the given subcommand(s)

Options:
  -v, --verbose  Enable verbose output
  -h, --help     Print help
  -V, --version  Print version
```