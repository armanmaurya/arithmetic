# Arithmetic

<p align="center">
  <img src="packaging/arithmetic.svg" width="128" height="128" alt="Arithmetic Logo">
</p>

<p align="center">
  <strong>A scientific calculator for Linux.</strong><br>
  Built with Rust and <a href="https://slint.dev">Slint</a>.
</p>

---

## Installation

### Arch Linux

```bash
sudo pacman -U ./arithmetic-*.pkg.tar.zst
```

### Debian / Ubuntu

```bash
sudo dpkg -i arithmetic-*.deb
```

### Fedora / RHEL / openSUSE

```bash
sudo dnf install ./arithmetic-*.rpm
```

### AppImage

```bash
chmod +x arithmetic-*.AppImage
./arithmetic-*.AppImage
```

### Build from Source

```bash
cargo build --release
```

---

## Skins

Arithmetic supports modular UI skins. Custom skins can be placed in:

* `~/.config/arithmetic/skins/<skin-id>/`
* `~/.local/share/arithmetic/skins/<skin-id>/`

### Bundled Skins

* `material-3`: Material Design layout with compact, standard, and large variants.
* `classic-desktop`: Traditional desktop calculator layout.
* `casio-fx991ms`: Scientific calculator layout.
* `ti-datamath-1977`: Vintage 1970s calculator layout.
* `braun-et66`: Minimalist Braun ET66 layout.

Each skin directory requires a `manifest.json` and a `skin.slint` file.

---

## Keyboard Shortcuts

| Key | Action |
| --- | --- |
| `0` - `9`, `.` | Input numbers and decimal point |
| `+`, `-`, `*`, `/` | Arithmetic operators |
| `^`, `%`, `!` | Power, percentage, factorial |
| `(`, `)` | Parentheses |
| `Enter` or `=` | Calculate result |
| `Backspace` | Delete character or token |
| `Esc` or `C` | Clear input |

---

## License

Distributed under the [MIT License](LICENSE).
