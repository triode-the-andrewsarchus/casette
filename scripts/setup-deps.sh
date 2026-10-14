#!/usr/bin/env bash
# Casette Dependency Setup Script
#
# Detects the Linux distribution and installs SageMath following the official
# SageMath documentation (https://doc.sagemath.org/html/en/installation/):
#   - Arch Linux: pacman -S sagemath
#   - Ubuntu / Debian: Miniforge / Conda environment 'sage' (user-local, no root needed)
#
# Guaranteed behaviors:
#   1. Idempotent: If SageMath is already available, does nothing and exits 0 immediately.
#   2. Root-safe: Never requests root credentials if Sage is already present, and
#      uses user-local Conda on Ubuntu so root credentials are never required.

set -euo pipefail

# Probe whether a given binary can run SageMath
probe_sage_bin() {
    local bin="$1"
    if [ -x "$bin" ] || command -v "$bin" >/dev/null 2>&1; then
        if "$bin" -c "import sage.all, sage.version; print(sage.version.version)" >/dev/null 2>&1; then
            return 0
        fi
    fi
    return 1
}

# Check if SageMath is already available in the environment
check_sage_available() {
    # 1. Explicit environment overrides
    if [ -n "${CASETTE_SAGE_PYTHON:-}" ] && probe_sage_bin "$CASETTE_SAGE_PYTHON"; then
        echo "$CASETTE_SAGE_PYTHON"
        return 0
    fi
    if [ -n "${CASETTE_SAGE_PATH:-}" ] && probe_sage_bin "$CASETTE_SAGE_PATH"; then
        echo "$CASETTE_SAGE_PATH"
        return 0
    fi

    # 2. 'sage' command in PATH
    if command -v sage >/dev/null 2>&1 && probe_sage_bin sage; then
        command -v sage
        return 0
    fi

    # 3. System 'python3' with sage.all
    if command -v python3 >/dev/null 2>&1 && probe_sage_bin python3; then
        command -v python3
        return 0
    fi

    # 4. Standard known paths (system packages, Conda environments)
    local candidate_paths=(
        "$HOME/miniforge3/envs/sage/bin/sage"
        "$HOME/miniforge3/envs/sage/bin/python"
        "$HOME/miniforge3/envs/sage/bin/python3"
        "$HOME/miniconda3/envs/sage/bin/sage"
        "$HOME/miniconda3/envs/sage/bin/python"
        "$HOME/.conda/envs/sage/bin/sage"
        "$HOME/.conda/envs/sage/bin/python"
        "$HOME/anaconda3/envs/sage/bin/sage"
        "/opt/conda/envs/sage/bin/sage"
        "/opt/miniforge3/envs/sage/bin/sage"
        "/usr/bin/sage"
        "/usr/local/bin/sage"
        "/opt/sagemath/sage"
    )

    for cand in "${candidate_paths[@]}"; do
        if [ -x "$cand" ] && probe_sage_bin "$cand"; then
            echo "$cand"
            return 0
        fi
    done

    # 5. Existing Conda / Mamba environment named 'sage'
    local conda_cmds=(
        "conda"
        "mamba"
        "$HOME/miniforge3/bin/conda"
        "$HOME/miniforge3/bin/mamba"
        "$HOME/miniconda3/bin/conda"
        "$HOME/anaconda3/bin/conda"
    )
    for c_cmd in "${conda_cmds[@]}"; do
        if command -v "$c_cmd" >/dev/null 2>&1 || [ -x "$c_cmd" ]; then
            if "$c_cmd" env list 2>/dev/null | grep -E '^[[:space:]]*sage[[:space:]]' >/dev/null 2>&1; then
                if "$c_cmd" run -n sage python -c "import sage.all" >/dev/null 2>&1; then
                    echo "$c_cmd (env: sage)"
                    return 0
                fi
            fi
        fi
    done

    return 1
}

# Detect the Linux distribution
detect_distro() {
    local os_id=""
    local os_like=""

    if [ -f /etc/os-release ]; then
        os_id=$(grep -E '^ID=' /etc/os-release | cut -d= -f2 | tr -d '"' | tr '[:upper:]' '[:lower:]')
        os_like=$(grep -E '^ID_LIKE=' /etc/os-release | cut -d= -f2 | tr -d '"' | tr '[:upper:]' '[:lower:]')
    fi

    case "$os_id" in
        arch|manjaro|endeavouros|artix|garuda)
            echo "arch"
            return 0
            ;;
    esac
    case "$os_like" in
        *arch*)
            echo "arch"
            return 0
            ;;
    esac

    case "$os_id" in
        ubuntu|debian|pop|linuxmint|elementary|zorin)
            echo "ubuntu"
            return 0
            ;;
    esac
    case "$os_like" in
        *ubuntu*|*debian*)
            echo "ubuntu"
            return 0
            ;;
    esac

    if command -v pacman >/dev/null 2>&1; then
        echo "arch"
        return 0
    elif command -v apt-get >/dev/null 2>&1; then
        echo "ubuntu"
        return 0
    fi

    echo "other"
    return 0
}

# Install SageMath on Arch Linux via pacman
install_arch() {
    echo "Detected Arch Linux / pacman-based system."
    echo "Installing sagemath via pacman (official SageMath documentation: https://doc.sagemath.org/html/en/installation/) ..."

    if [ "$(id -u)" -eq 0 ]; then
        pacman -S --needed --noconfirm sagemath
    else
        if command -v sudo >/dev/null 2>&1; then
            echo "Requesting sudo privileges to run 'pacman -S --needed sagemath'..."
            sudo pacman -S --needed sagemath
        else
            echo "Error: root privileges (or sudo) required to run pacman." >&2
            exit 1
        fi
    fi
}

# Find an existing conda or mamba executable, or install Miniforge3
find_or_install_conda() {
    # 1. Existing mamba or conda on PATH
    if command -v mamba >/dev/null 2>&1; then
        command -v mamba
        return 0
    fi
    if command -v conda >/dev/null 2>&1; then
        command -v conda
        return 0
    fi

    # 2. Common install prefixes
    local prefixes=(
        "$HOME/miniforge3"
        "$HOME/miniconda3"
        "$HOME/anaconda3"
        "/opt/conda"
        "/opt/miniforge3"
    )
    for p in "${prefixes[@]}"; do
        if [ -x "$p/bin/mamba" ]; then
            echo "$p/bin/mamba"
            return 0
        elif [ -x "$p/bin/conda" ]; then
            echo "$p/bin/conda"
            return 0
        fi
    done

    # 3. Not found: Install Miniforge3 as recommended by SageMath documentation
    echo "No Conda installation found. Installing Miniforge3 following https://doc.sagemath.org/html/en/installation/ ..." >&2
    local os_kernel
    local arch
    os_kernel="$(uname -s)"
    arch="$(uname -m)"
    local installer_url="https://github.com/conda-forge/miniforge/releases/latest/download/Miniforge3-${os_kernel}-${arch}.sh"
    local install_dir="$HOME/miniforge3"
    local tmp_sh
    tmp_sh="$(mktemp /tmp/miniforge-XXXXXX.sh)"

    echo "Downloading Miniforge installer from: ${installer_url}" >&2
    if command -v curl >/dev/null 2>&1; then
        curl -fsSL "$installer_url" -o "$tmp_sh"
    elif command -v wget >/dev/null 2>&1; then
        wget -qO "$tmp_sh" "$installer_url"
    else
        echo "Error: neither curl nor wget found to download Miniforge." >&2
        rm -f "$tmp_sh"
        exit 1
    fi

    echo "Installing Miniforge3 into ${install_dir} (user-local, no root needed)..." >&2
    bash "$tmp_sh" -b -p "$install_dir"
    rm -f "$tmp_sh"

    if [ -x "$install_dir/bin/mamba" ]; then
        echo "$install_dir/bin/mamba"
    elif [ -x "$install_dir/bin/conda" ]; then
        echo "$install_dir/bin/conda"
    else
        echo "Error: Miniforge installation failed to create executable binary." >&2
        exit 1
    fi
}

# Install SageMath on Ubuntu / Debian using Conda
install_ubuntu_conda() {
    echo "Detected Ubuntu / Debian-based distribution."
    echo "Setting up SageMath via Conda following https://doc.sagemath.org/html/en/installation/ ..."

    local conda_tool
    conda_tool="$(find_or_install_conda)"
    echo "Using Conda executable: $conda_tool"

    # Check if 'sage' environment already exists
    if "$conda_tool" env list 2>/dev/null | grep -E '^[[:space:]]*sage[[:space:]]' >/dev/null 2>&1; then
        echo "Conda environment 'sage' already exists."
    else
        echo "Creating Conda environment 'sage' with SageMath (python=3.12)..."
        echo "This may take a few minutes while conda-forge packages are downloaded and installed."
        "$conda_tool" create -y -n sage -c conda-forge sage python=3.12
    fi

    # Verify and locate the newly configured sage binary
    local conda_base
    conda_base="$("$conda_tool" info --base 2>/dev/null || dirname "$(dirname "$conda_tool")")"
    local sage_candidate="$conda_base/envs/sage/bin/sage"
    if [ -x "$sage_candidate" ]; then
        echo "SageMath successfully installed at: $sage_candidate"
    else
        echo "SageMath environment 'sage' created successfully."
    fi
}

main() {
    echo "=== Casette Dependency Setup ==="

    # 1. Check if SageMath is already available
    if found_sage=$(check_sage_available); then
        echo "SageMath is already available at: ${found_sage}"
        local version_str
        version_str=$("$found_sage" -c "import sage.version; print(sage.version.version)" 2>/dev/null || echo "detected")
        echo "Detected SageMath version: ${version_str}"
        echo "Nothing to do. Exiting setup."
        exit 0
    fi

    echo "SageMath not currently available in environment."

    # 2. Detect Linux distribution
    local distro
    distro=$(detect_distro)
    echo "Detected distribution category: ${distro}"

    # 3. Perform appropriate installation
    case "$distro" in
        arch)
            install_arch
            ;;
        ubuntu)
            install_ubuntu_conda
            ;;
        *)
            echo "Non-Arch/Ubuntu Linux detected. Following SageMath recommendation for other Linux distributions:"
            echo "Installing via Conda from conda-forge (https://doc.sagemath.org/html/en/installation/)..."
            install_ubuntu_conda
            ;;
    esac

    # 4. Final verification
    if found_sage=$(check_sage_available); then
        echo "=== Setup complete! SageMath is available at: ${found_sage} ==="
    else
        echo "Warning: Setup completed, but automated check could not immediately run Sage."
        echo "If you installed via Conda, you may need to run 'conda activate sage' or set CASETTE_SAGE_PYTHON."
    fi
}

if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    main "$@"
fi
