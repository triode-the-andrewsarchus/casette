# Linux Distribution Detection & Dependency Setup Plan

This document details the architecture and operational contract of Casette's
automated distribution detection and dependency management system.

## Objective & Upstream Standards

Casette relies on a local SageMath environment (Sage 10+ / Python 3 with `sage.all`
and `matplotlib`). Setting up SageMath across Linux distributions must follow the
canonical upstream documentation at:
**<https://doc.sagemath.org/html/en/installation/>**

Specific requirements:
1. **Arch Linux**: Install `sagemath` via `pacman` (`sudo pacman -S --needed sagemath`).
2. **Ubuntu / Debian**: Install via Conda from `conda-forge` (`conda create -n sage sage python=3.12`).
   If Conda/Mamba is not present, automatically install Miniforge3 into `$HOME/miniforge3`.
3. **Other Linux Distributions**: Follow SageMath's recommendation to install via Conda
   from `conda-forge`.
4. **Idempotency**: If SageMath is already available and functional, do not attempt
   reinstallation or package modifications.
5. **Non-Intrusive Make / Root Safety**: Running standard `make` or `cargo build`
   must **never** require root credentials. On Ubuntu, Conda runs entirely in user
   space without `sudo`. On Arch, `sudo` is only invoked if `make setup` is run
   explicitly on a system lacking SageMath.

---

## Architecture

```text
┌────────────────────────────────────────────────────────┐
│                      make build                        │
│   (runs 'check-sage': fast non-blocking presence probe) │
└───────────────────────────┬────────────────────────────┘
                            │
              Sage missing? └──> [Emits note to run 'make setup']
                                 (Proceeds to cargo build, NO root)

┌────────────────────────────────────────────────────────┐
│                      make setup                        │
│               (scripts/setup-deps.sh)                  │
└───────────────────────────┬────────────────────────────┘
                            │
               Is Sage already available?
              (PATH, Conda envs, system paths)
                            │
            ┌───────────────┴───────────────┐
           YES                              NO
            │                               │
     [Print version & exit 0]        [Detect distro]
     (Zero root, no-op)                     │
                               ┌────────────┴────────────┐
                           Arch Linux                 Ubuntu / Debian
                               │                             │
                   [sudo pacman -S sagemath]         [Conda available?]
                                                             │
                                                    ┌────────┴────────┐
                                                   YES                NO
                                                    │                 │
                                                    │        [Install Miniforge3]
                                                    │        (user-local, no root)
                                                    └────────┬────────┘
                                                             │
                                                  [mamba/conda create -y]
                                                  [-n sage -c conda-forge]
                                                  [sage python=3.12]
```

---

## Components

### 1. Pre-Flight Availability Check (`check_sage_available`)
Before invoking any package manager or downloading installers, `scripts/setup-deps.sh`
probes the environment using:
1. Explicit environment overrides: `$CASETTE_SAGE_PYTHON`, `$CASETTE_SAGE_PATH`
2. `sage` binary on `$PATH`
3. System `python3` with `sage.all` import capability
4. Standard filesystem candidate paths:
   - `$HOME/miniforge3/envs/sage/bin/sage`
   - `$HOME/miniforge3/envs/sage/bin/python`
   - `$HOME/miniconda3/envs/sage/bin/sage`
   - `$HOME/miniconda3/envs/sage/bin/python`
   - `$HOME/.conda/envs/sage/bin/sage`
   - `$HOME/anaconda3/envs/sage/bin/sage`
   - `/usr/bin/sage`
   - `/usr/local/bin/sage`
   - `/opt/sagemath/sage`
5. Existing Conda/Mamba environments named `sage`.

If any candidate successfully executes `import sage.all, sage.version`, setup prints
the detected path and version, and exits 0 immediately.

### 2. Distribution Detection (`detect_distro`)
Inspects `/etc/os-release` parsing `ID` and `ID_LIKE`:
- **Arch Linux & derivatives**: Matches `arch`, `manjaro`, `endeavouros`, `artix`,
  `garuda`, or distros reporting `*arch*` in `ID_LIKE`. Fallback: `command -v pacman`.
- **Ubuntu & Debian derivatives**: Matches `ubuntu`, `debian`, `pop`, `linuxmint`,
  `elementary`, `zorin`, or distros reporting `*ubuntu*` or `*debian*` in `ID_LIKE`.
  Fallback: `command -v apt-get`.
- **Others**: Defaults to Conda per SageMath upstream recommendation.

### 3. Ubuntu & Conda Setup Pipeline
On Ubuntu:
1. Checks for existing `mamba` or `conda` in `PATH` or standard prefixes (`$HOME/miniforge3`,
   `$HOME/miniconda3`, etc.).
2. If missing, downloads Miniforge3 from GitHub releases (`Miniforge3-$(uname -s)-$(uname -m).sh`)
   and executes unattended batch installation into `$HOME/miniforge3`.
3. Invokes `mamba` (or `conda`) to create environment `sage` with `sage python=3.12` from
   `conda-forge`.
4. The entire process executes within the user's home directory and requires zero `sudo`
   or root privileges.

### 4. Arch Linux Setup Pipeline
On Arch:
1. Runs `pacman -S --needed sagemath`.
2. Automatically detects if running as `root` (e.g. Docker/CI container) to omit `sudo`.
3. As non-root, prompts for `sudo` only when `make setup` is explicitly triggered.

### 5. Makefile Integration
- `all: build` remains the default goal.
- `make check-sage` runs before `cargo build` in `build: check-sage`. It performs a
  sub-50ms non-blocking check and outputs a helpful note if Sage is missing, without
  halting the build or requesting root credentials.
- `make setup` (and alias `make deps`) invokes `scripts/setup-deps.sh`.
- The Makefile exports a default fallback `CASETTE_SAGE_PYTHON` pointing to
  `$(HOME)/miniforge3/envs/sage/bin/python` if present on disk.

### 6. In-App & Kernel Discovery Synchronization
`casette-kernel::SageDiscovery` and `casette-gui::TextureCache` candidate lists are
expanded to inspect `$HOME/miniforge3/envs/sage/bin/sage` and Python equivalents.
When Sage is installed via Conda, launching `casette` (or running `cargo test`)
automatically discovers the runtime environment without requiring manual shell
activation.
