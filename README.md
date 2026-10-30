<p align="center"><img src="assets/banner.png" alt="mdview: tiny, fast, native Markdown viewer for Windows"></p>


A tiny, fast, native Markdown viewer for Windows. Open a folder of `.md` files without launching an editor.

- Single `mdv.exe` (~850 KB), starts in about a tenth of a second
- Sidebar with every `.md` in the folder (subfolders included) and a quick filter
- Live reload when the file changes, scroll position kept
- Tables, task lists, strikethrough, footnotes
- Links between `.md` files open inside the app; web links open in your browser
- Light/dark theme follows Windows
- Uses the WebView2 runtime that ships with Windows 11 (no extra install)

## Usage

```
mdv                 # open the default folder
mdv C:\path\docs    # open a folder
mdv notes.md        # open a file (its folder fills the sidebar)
```

`F5` re-reads the folder and the current file.

The default folder is set by `DEFAULT_ROOT` in `src/main.rs`, and the WebView2 profile folder by `DATA_DIR`. Change both to suit your machine and rebuild.

## Install

Download `mdv.exe` from the [latest release](https://github.com/bps2414/mdview/releases/latest) and put it anywhere on your `PATH`.

To list it under **Open with** for `.md` files (current user only):

```powershell
$exe = 'C:\path\to\mdv.exe'
$app = 'HKCU:\Software\Classes\Applications\mdv.exe'
New-Item "$app\shell\open\command" -Force | Out-Null
New-Item "$app\SupportedTypes" -Force | Out-Null
Set-ItemProperty "$app\shell\open\command" -Name '(default)' -Value "`"$exe`" `"%1`""
Set-ItemProperty "$app\SupportedTypes" -Name '.md' -Value ''
New-Item 'HKCU:\Software\Classes\.md\OpenWithList\mdv.exe' -Force | Out-Null
```

## Build

Requires Rust (stable).

```
cargo build --release
```

The binary is `target\release\mdv.exe`.

## Limitations

- Images with relative paths inside Markdown files are not loaded
- No syntax highlighting in code blocks
- Windows only (WebView2)

## License

MIT
