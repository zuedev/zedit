> [!WARNING]  
> This is a legacy project never intended for public use or scrutiny, use at your own risk!

# zedit

> Fast, slim, dependency-free editing with syntax highlighting and directory browsing.

## Abstract

**zedit** is a lightning-fast, dependency-free text editor built for constrained environments and rapid workflows. Prioritizing an ultra-low memory footprint, it provides a responsive editing experience on everything from powerful workstations to minimal server instances. It strips away the non-essentials (like mouse support and plugin systems) to focus on what matters: syntax highlighting, file navigation, and raw speed.

## Features

- **Syntax Highlighting**: Supports a wide range of programming languages with built-in syntax highlighting.
- **Directory Browsing**: Navigate your file system with ease using the integrated directory browser.
- **Minimalist Interface**: A clean, distraction-free interface that keeps you focused on your code.
- **Cross-Platform**: Runs on Windows, macOS, and Linux without any dependencies
- **Extremely Fast**: Optimized for speed, making it ideal for quick edits and large files.
- **Low Memory Usage**: Designed to run efficiently even on systems with limited resources.

## Installation

We support multiple methods of installation.

<!-- ### Quick Install Script

We provide a convenient installation script hosted on our website. You can run it with the following command:

**bash:**

```bash
curl https://zue.dev/zedit | bash
```

**PowerShell:**

```powershell
iex (iwr https://zue.dev/zedit)
``` -->

### Precompiled Binaries

Check the [releases](https://github.com/zuedev/zedit/releases) page for precompiled binaries for your platform.

### Building from Source

As the project uses Rust, you can build it from source using Cargo. First, ensure you have Rust installed, then run:

```bash
git clone https://github.com/zuedev/zedit.git && cd zedit && cargo build --release
```

This will create an optimized binary in the `target/release` directory. You can move this binary to a location in your system's PATH for easy access.

## Usage

```bash
zedit [options] [file/directory]
```

### Options

- `-h, --help`: Show help message and exit.
- `-v, --version`: Show version information and exit.

## License

This project is dedicated to the public domain. For more information, see the [LICENSE](LICENSE) file.
