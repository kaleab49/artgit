**ArtGit** is a **Git-like version control system for creative files** such as images, videos, audio, 3D models, and documents. It is designed for **designers, video editors, and other creatives** who want **version control without the complexity of Git**.  

With ArtGit, you can track your file changes, rollback versions, and visualize your project history — all locally, fast, and efficiently.

---

## Features 🚀

- **Initialize a repository**
  ```bash
  artgit init

Create a new project repository with all necessary metadata.

Commit file changes

artgit commit -m "Added character design v1"

Detects new or modified files, compresses them efficiently, and stores a versioned snapshot.

Check repository status

artgit status

Shows new, modified, and unchanged files in your project.

View commit history

artgit log

Displays all past commits with timestamps and commit messages.

Efficient storage

SHA256 hash-based versioning

Compressed binary storage using lz4 or zstd

Metadata stored in JSON for easy inspection

Terminal interface (optional)

ASCII commit timeline

Visual overview of project versions

Installation 🛠

Clone the repository

git clone https://github.com/yourusername/artgit.git
cd artgit

Build the project with Cargo

cargo build --release

Run ArtGit

./target/release/artgit init
How It Works ⚙️

ArtGit creates a hidden .artgit/ folder in your project directory:

.artgit/
 ├─ objects/         # compressed file versions
 ├─ metadata.json    # commit history and file info
 └─ branches.json    # optional branch management

Files are hashed to avoid duplicates

Changes are stored incrementally for efficiency

Metadata tracks every commit, timestamp, and file state

Example Workflow ✨
# Initialize a repository
artgit init

# Add changes and commit
artgit commit -m "First version of character design"

# Check status
artgit status

# View commit history
artgit log
Future Roadmap 🗺

Branching and merging for creative workflows

Peer-to-peer sync over LAN

File previews in TUI

Visual diff for images/videos

Web or desktop GUI

Why ArtGit? 💡

Designed specifically for creative professionals, not just developers

Efficient storage for large binary files

Lightweight, local-first, and fast

Extensible — can evolve into a full GitHub alternative for creatives

Contributing 🤝

Contributions are welcome!

Fork the repo

Create a branch (git checkout -b feature/my-feature)

Submit a pull request

License 📝

MIT License – see LICENSE
 for details.

ArtGit – version control that works for creatives, not just coders! 🎨


---

If you want, I can also make a **“demo GIF + ASCII screenshots section”** for the README that shows commits, logs, and status — it’ll make it **instantly impressive to anyone visiting your GitHub**.  

Do you want me to do that?