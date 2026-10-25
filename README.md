# Arithmetic

<p align="center">
  <img src="packaging/arithmetic.svg" width="128" height="128" alt="Arithmetic Logo">
</p>

<p align="center">
  <strong>A modern, responsive Material 3 scientific calculator for Linux.</strong><br>
  Built with Rust and <a href="https://slint.dev">Slint</a>. Tailored for modern desktop environments and Wayland window managers like Hyprland.
</p>

---

## Installation

### Arch Linux
Download the `.pkg.tar.zst` package from [Releases](../../releases) and install:
```bash
sudo pacman -U ./arithmetic-*.pkg.tar.zst
```
*(Or via AUR once published: `yay -S arithmetic`)*

### Universal AppImage
Download the prebuilt AppImage from [Releases](../../releases), make it executable, and run:
```bash
chmod +x arithmetic-*.AppImage
./arithmetic-*.AppImage
```

### Debian / Ubuntu
Download the `.deb` package from [Releases](../../releases) and install:
```bash
sudo dpkg -i arithmetic-*.deb
```

### Fedora, openSUSE & RHEL
Download the `.rpm` package from [Releases](../../releases) and install:
```bash
sudo dnf install ./arithmetic-*.rpm
```

---

## Layout Skins

Arithmetic supports full runtime UI layout skins. Users and theme creators can drop custom skins into `~/.config/arithmetic/skins/<skin-id>/` without recompiling the application.

### Bundled Skins
* **Material 3 (`material-3`)**: Responsive smartphone-style layout with scientific drawer and collapsible history panel.
* **Classic Desktop (`classic-desktop`)**: Retro 90s desktop calculator with 3D beveled buttons and vintage olive-green LCD display.
* **Casio Scientific (`casio-scientific`)**: Realistic handheld hardware calculator with dual-line dot-matrix LCD and dedicated scientific keys.
* **Compact Bar (`compact-bar`)**: Ultra-compact minimalist floating bar designed for tiling window managers like Hyprland/i3.
* **Cyberpunk Neon (`cyberpunk-neon`)**: High-contrast sci-fi terminal layout with glowing neon cyan/magenta HUD accents.
* **Skeuomorphic (`skeuomorphic`)**: Iconic Dieter Rams Braun ET66 tactile design with 3D molded keycaps, physical mechanical press-down action, sunken glass LCD, and signature glossy amber equals key.

### Creating Custom Skins
Each skin directory contains:
* `manifest.json`: Metadata (`id`, `name`, `author`, `version`, `main: "skin.slint"`)
* `skin.slint`: Root Slint component exporting `AppWindow inherits Window` implementing standard state properties (`expression`, `cursor-byte-offset`, `preview-result`, `use-radians`) and callbacks (`press-key`, `expression-edited`, `toggle-angle-unit`, `switch-skin`).

---

## Keyboard Shortcuts

| Key | Action |
| :--- | :--- |
| `0` – `9`, `.` | Input numbers and decimal point |
| `+`, `-`, `*`, `/` | Operators (`+`, `−`, `×`, `÷`) |
| `^`, `%`, `!` | Power, percentage, factorial |
| `(`, `)` | Smart parentheses insertion |
| `Enter` or `=` | Calculate result |
| `Backspace` | Caret-aware token backspace |
| `Esc` or `C` | All Clear (`AC`) / dismiss dialogs |

---

## License
Distributed under the [MIT License](LICENSE).
