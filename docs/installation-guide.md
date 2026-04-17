# Installation Guide

## Why Home Manager?

Home Manager is the only recommended way to install Ephemera AI. This is a deliberate design choice rooted in two core principles:

- **Explicit configuration**: Every setting is declared in Nix — no hidden defaults, no scattered config files. You can always trace what is configured and why.
- **Atomic updates**: Ephemera AI manages its own infrastructure. After deployment, it takes full ownership of the home-manager configuration, applying changes atomically — they either fully apply or fully roll back, never leaving the system in a half-updated state.

For these reasons, home-manager is not just a convenience — it is the foundation of Ephemera AI's self-management capability.

## Prerequisites

### System Requirements

| Requirement                  | Details                                                                                |
| ---------------------------- | -------------------------------------------------------------------------------------- |
| **OS**                       | Linux (x86_64 or aarch64)                                                              |
| **Container virtualization** | Must be enabled in system settings (e.g. `systemd-nspawn`, `subuid`/`subgid` mappings) |
| **Podman**                   | Required for running containerized services (MySQL, etc.)                              |

## Prerequisites

### Non-NixOS Linux

#### Install Podman

Install podman through your distribution's package manager. For example:

```bash
# Debian/Ubuntu
sudo apt install podman

# Fedora
sudo dnf install podman

# Arch Linux
sudo pacman -S podman
```

Verify container virtualization works:

```bash
podman run --rm docker.io/library/alpine echo "OK"
```

#### Install Nix

Follow the [Nix installation guide](https://nixos.org/download) to install the Nix package manager.

After installation, ensure the following are configured in `nix.conf` — flakes are required for this project:

```bash
# /etc/nix/nix.conf
experimental-features = nix-command flakes
trusted-users = ephemera
```

Verify:

```bash
nix --version
nix flake --help  # flakes available
```

#### Install Home Manager

Follow the [Home Manager standalone installation](https://nix-community.github.io/home-manager/index.html#sec-install-standalone) guide.

Verify:

```bash
home-manager --version
```

### NixOS

NixOS already includes Nix. You need to enable [Flakes](https://wiki.nixos.org/wiki/Flakes) and set up [Podman](https://wiki.nixos.org/wiki/Podman) for container virtualization. The dedicated user is declared directly in your NixOS configuration via `users.users`. For Home Manager installation, refer to the [Home Manager manual](https://nix-community.github.io/home-manager/).

The [NixOS Wiki](https://wiki.nixos.org/) covers all of these topics in detail.

## Creating a Dedicated User (Recommended)

Since Ephemera AI takes full ownership of home-manager configuration after deployment — managing all packages, services, and dotfiles — we strongly recommend creating a dedicated system user. This isolates Ephemera AI's environment from your personal user account, both for security and for configuration cleanliness.

On non-NixOS systems:

```bash
# Create a dedicated user (adjust username as needed)
sudo useradd -m -s /bin/bash ephemera

# Allow podman access (if using rootless containers)
sudo usermod -aG podman ephemera

# Switch to the new user
sudo -iu ephemera

# Initialize Nix for this user (if not already available system-wide)
nix --version
```

## Installation

### Option A: Initialize from Nix Template (Recommended)

If this is a fresh home-manager setup, use the built-in flake template to generate a complete initial configuration:

```bash
# From the ephemera user's home directory
cd ~
nix flake init -t github:ImitationGameLabs/ephemera-ai
```

This creates three files in the current directory:

| File              | Purpose                                                                  |
| ----------------- | ------------------------------------------------------------------------ |
| `home.nix`        | Base home-manager configuration (username, state version)                |
| `env.nix`         | Development packages (jq, tree, devenv, etc.)                            |
| `ephemera-ai.nix` | Ephemera AI service definitions (agent, memory, events, chat, databases) |
| `flake.nix`       | Flake configuration with all required inputs and modules                 |

Review and edit the generated files — especially `ephemera-ai.nix` to set your API keys, passwords, and service ports.

Then apply the configuration:

```bash
home-manager switch --flake .#simplex
```

> **Note**: The flake template defines the home configuration as `simplex`. Rename it to match your username by editing `flake.nix` if needed.

### Option B: Integrate into Existing Home Manager Setup

If you already have a home-manager configuration and do not want to start fresh, you can manually integrate Ephemera AI into your existing flake.

**1. Add the ephemera-ai input to your `flake.nix`:**

```nix
inputs = {
  # ... your existing inputs ...

  ephemera-ai = {
    url = "github:ImitationGameLabs/ephemera-ai";
    inputs.nixpkgs.follows = "nixpkgs";
  };
};
```

**2. Add the home-manager module to your modules list:**

```nix
modules = [
  # ... your existing modules ...

  ephemera-ai.homeManagerModules.default
];
```

**3. Copy the service configuration file:**

```bash
# Copy the template service configuration into your home-manager config directory
cp templates/default/ephemera-ai.nix ~/path/to/your/config/
```

Then add `./ephemera-ai.nix` to your modules list in `flake.nix`.

**4. Review and apply:**

Edit `ephemera-ai.nix` to configure API keys, passwords, and other settings for your environment, then run:

```bash
home-manager switch
```

## Post-Installation

After a successful `home-manager switch`, all Ephemera AI services should be running as systemd user services. Verify with:

```bash
epha-ctl status
```


See [Development Guide](development-guide.md) for next steps on building and developing Ephemera AI.
