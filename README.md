# Arithmetic

<p align="center">
  <img src="packaging/arithmetic.svg" width="128" height="128" alt="Arithmetic Logo">
</p>

<p align="center">
  <strong>A modern, responsive Material 3 scientific calculator for Linux.</strong><br>
  Built with Rust and <a href="https://slint.dev">Slint</a>. Tailored for modern desktop environments and Wayland window managers like Hyprland.
</p>

---

## Features

- **Material 3 / Material You Design**: Seamless light, dark, and system default themes with dynamic squircle morph animations.
- **Pull-Down Persistent History**: Swipe down or tap the drag pill to open the calculation history tape; entries persist across app restarts and can be clicked to reuse past expressions or results.
- **Editable Expression & Caret Positioning**: Click anywhere in the expression to place the cursor and insert or delete characters mid-expression.
- **No Ellipsis Overflow**: Expressions smoothly scroll horizontally with zero padding clipping brackets or descenders.
- **Expandable Scientific Panel**:
  - Portrait mode: Smooth expandable drawer with trigonometry, inverse trig (`INV`), logarithms (`ln`, `log`), powers (`x²`, `eˣ`, `10ˣ`, `^`), factorial (`!`), and constants (`π`, `e`).
  - Landscape mode: Automatically switches to an Android-style dual-pane layout (3×4 scientific panel on left, 4×5 numpad on right) when window width ≥ 640px.
- **Native Hyprland Integration**: Automatically launches floating and centered in Hyprland without tiling conflicts.
- **Self-Contained Native Binary**: Compiles directly into a single stripped native executable with zero external runtime UI dependencies.

---

## Installation & Packaging

### 1. Arch Linux / AUR
Build and install using Arch's official `makepkg`:
```bash
cd packaging
makepkg -si
```
Or via an AUR helper:
```bash
yay -S arithmetic
```

### 2. Universal AppImage (`cargo appimage`)
Build the standalone AppImage:
```bash
cargo appimage
```
Or run the prebuilt AppImage from [Releases](../../releases):
```bash
chmod +x arithmetic-*.AppImage
./arithmetic-*.AppImage
```

### 3. Debian / Ubuntu (`cargo deb`)
Build the `.deb` package:
```bash
cargo deb
```
Install the package:
```bash
sudo dpkg -i target/debian/arithmetic_*.deb
```

### 4. Fedora, openSUSE & RHEL (`cargo generate-rpm`)
Build the `.rpm` package:
```bash
cargo generate-rpm
```
Install the package:
```bash
sudo dnf install target/generate-rpm/arithmetic-*.rpm
```

### 5. Direct Cargo Install
Install directly to `~/.cargo/bin`:
```bash
cargo install --path .
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
