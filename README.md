# git trunk

git trunk is a CLI tool for managing repository-wide documentation in a .trunk directory within a Git repository, stored under refs/trunk/main in the main repository.

Use it to hold information such as issues, bugs, changelog etc. These files can be maintained repowide and not be branch specific. 



```
❯ git trunk
Git Trunk CLI for managing repository-wide documents

Usage: git-trunk <COMMAND>

Commands:
  init   Initializes the git-trunk in the current repository
  sync   Syncs changes from .trunk to the main repository
  clone  Clones the trunk from refs/trunk/main into .trunk
  push   Pushes the objects from refs/trunk/main to remote (default origin)
  hooks  Manages Git hooks for git-trunk
  help   Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
```