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
