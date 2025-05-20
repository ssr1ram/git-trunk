# git trunk

git trunk is a CLI tool for managing repository-wide documentation in a .trunk directory within a Git repository, stored under refs/trunk/main in the main repository.

It helps you 
- Hold information that is common across all branches such as issues, bugs, changelog, history etc. These files can be maintained repowide and not be branch specific.
- Get steganographic i.e. conceal information in a public git repo that is non-evident.


## About git-trunk
**`git-trunk` CLI: Core Functionality**

`git-trunk` is a command-line interface tool designed to manage repository-wide documents and metadata, which are stored within a dedicated `.trunk/<store_name>` directory inside a Git repository. These "stores" are then tracked in the main repository using a special Git reference, typically `refs/trunk/<store_name>`. The primary aim is to keep certain information (like issues, changelogs, design documents) separate from the main project's branching history but still versioned and shareable within the same repository. It also offers a way to "hide" these files from the working directory on demand (steganography).

**Global Options:**

*   `-v, --verbose`: Enables detailed debug logging.
*   `-r, --remote <REMOTE>`: Specifies the Git remote to interact with (default: `origin`).
*   `-s, --store <STORE>`: Specifies the name of the "trunk store" to operate on (default: `main`). Most commands target a specific store.

**Key Commands and Their Actions (Per Store):**

1.  **`init`** (`commands::init.rs`):
    *   Initializes a new trunk store.
    *   Ensures the current directory is a Git repository.
    *   Adds `.trunk` to the main repository's `.gitignore` file if not already present.
    *   Creates the `.trunk/` parent directory if it doesn't exist.
    *   Creates the specific `.trunk/<store_name>` directory.
    *   If `--force` is used and the directory exists, it's removed and recreated.
    *   Creates a `readme.md` file inside `.trunk/<store_name>`.
    *   Initializes a new Git repository within `.trunk/<store_name>`.
    *   Adds and commits the `readme.md` in this new inner Git repository.

2.  **`commit`** (`commands::commit.rs`):
    *   Commits changes made within an existing `.trunk/<store_name>` directory to the main repository's `refs/trunk/<store_name>` reference.
    *   Checks if `.trunk/<store_name>` exists and is a Git repository.
    *   Checks for uncommitted changes within `.trunk/<store_name>`.
    *   If changes exist (and not `--force`), prompts the user to stage and commit them within the `.trunk/<store_name>` repository. The commit message can be provided via `-m` or defaults to a standard message.
    *   Retrieves the latest commit hash from the `main` branch of the `.trunk/<store_name>` repository.
    *   Fetches the objects from the `.trunk/<store_name>` repository into a temporary branch in the main repository.
    *   Updates (or creates) the `refs/trunk/<store_name>` reference in the main repository to point to this fetched commit hash.
    *   Cleans up the temporary branch.

3.  **`checkout`** (`commands::checkout.rs`):
    *   "Checks out" or materializes a trunk store from the main repository's `refs/trunk/<store_name>` reference into the local `.trunk/<store_name>` working directory.
    *   If `refs/trunk/<store_name>` doesn't exist locally, it attempts to find and fetch it from the specified remote.
    *   Ensures `.trunk` is in `.gitignore`.
    *   Creates the `.trunk/` and `.trunk/<store_name>` directories if they don't exist.
    *   If `.trunk/<store_name>` already exists:
        *   If `--force` is used, it removes the existing directory.
        *   Otherwise, it prompts the user to overwrite.
    *   Initializes a Git repository in `.trunk/<store_name>`.
    *   Fetches the commit history from the main repository's `refs/trunk/<store_name>` into a temporary ref within the `.trunk/<store_name>` repository.
    *   Resets the `main` branch of the `.trunk/<store_name>` repository to this fetched commit.
    *   Ensures `HEAD` points to `main` in the `.trunk/<store_name>` repository.
    *   Cleans up the temporary ref.

4.  **`push`** (`commands::push.rs`):
    *   Pushes the main repository's local `refs/trunk/<store_name>` reference to the specified remote repository.
    *   Verifies that `refs/trunk/<store_name>` exists locally.
    *   Executes `git push <remote_name> refs/trunk/<store_name>:refs/trunk/<store_name>`.

5.  **`hooks`** (`commands::hooks.rs`):
    *   Manages Git hooks for a specific trunk store to automate `commit` and `push` operations.
    *   Operates within the main repository's `.git/hooks` directory.
    *   **Post-commit hook**: Can install a hook that automatically runs `git trunk commit --force --store <store_name>` after a commit in the main repository.
    *   **Pre-push hook**: Can install a hook that automatically attempts to `git push <remote_name> refs/trunk/<store_name>:refs/trunk/<store_name>` (or more generically, `git trunk push --store <store_name> --remote <remote_name>`) when the main branch of the main repository is pushed.
    *   Prompts the user before overwriting existing hooks unless `--force` is used.

6.  **`stegano`** (`commands::stegano.rs`):
    *   Removes the specified `.trunk/<store_name>` working directory from the filesystem.
    *   If this action results in the parent `.trunk/` directory becoming empty, `stegano` will also:
        *   Remove the empty `.trunk/` directory.
        *   Remove the `.trunk` entry from the main repository's `.gitignore` file.
    *   This command **only affects the working directory**; it does not delete the `refs/trunk/<store_name>` Git reference.

7.  **`delete`** (`commands::delete.rs`):
    *   Completely removes all traces of a specific git-trunk store.
    *   Prompts for user confirmation due to its destructive nature.
    *   Removes the local `.trunk/<store_name>` working directory.
    *   Deletes the local `refs/trunk/<store_name>` reference from the main repository.
    *   Deletes the `refs/trunk/<store_name>` reference from the specified remote repository.
    *   If the parent `.trunk/` directory becomes empty after removing `.trunk/<store_name>`, it is also removed (but `.gitignore` entry for `.trunk` is not touched by this command, as other stores might still exist or be intended).

8.  **`info`** (`commands::info.rs`):
    *   Displays information about the git-trunk setup and specified/discovered stores.
    *   Can operate in two modes:
        *   Default: Shows info for the store specified by `--store` (or "main"), and also discovers other stores present locally (in `.trunk/` or as `refs/trunk/*`).
        *   `--all`: Discovers all stores present on the remote under `refs/trunk/*` and displays information for each.
    *   For each store, it shows:
        *   Local `.trunk/<store_name>` directory: existence, whether it's a Git repo, last commit hash/date, and status (uncommitted changes).
        *   Main repository `refs/trunk/<store_name>`: existence, last commit hash/date.
        *   Remote repository `refs/trunk/<store_name>`: existence on remote, commit hash.

**Utility:**

*   `utils.rs`: Contains a `run_git_command` helper function used by all commands to execute Git commands, manage verbose output, and perform a basic check for Git availability.



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
🐘 ✓ Step 1: Repository root found at <dirpath>
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

### make it steganographic
```sh
❯ git trunk stegano
🐘 ✓ Step 1: Confirmed inside a Git repository
🐘 ✓ Step 2: Repository root found at <dirpath>
🐘 ✓ Step 3: Removed .trunk from .gitignore
🐘 ✓ Step 4: .trunk directory removed
🐘 ✅ Stegano completed successfully: All traces of .trunk removed
```

## delete refs/trunk/main 
```sh
❯ git trunk delete  
🐘︖ This will delete .trunk, its .gitignore entry, and refs/trunk/main locally and on the remote (origin). Continue? [y/N]: y
🐘 ✓ Step 1: User confirmed deletion
🐘 ✓ Step 2: Confirmed inside a Git repository
🐘 ✓ Step 3: Repository root found at <dirpath>
🐘 ✓ Step 4: Removed .trunk from .gitignore
🐘 ✓ Step 5: .trunk directory removed
🐘 ✓ Step 6: Local refs/trunk/main deleted
🐘 ✓ Step 7: Remote refs/trunk/main deleted on origin
🐘 ✅ Delete completed successfully: All traces of git-trunk removed
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
  stegano   Removes all traces of .trunk from the main repository
  delete    Removes all traces of git-trunk, including .trunk and refs/trunk/main locally and remotely
  help      Print this message or the help of the given subcommand(s)

Options:
  -v, --verbose  Enable verbose output
  -h, --help     Print help
  -V, --version  Print version
```