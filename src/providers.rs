//! implements different clipboard types
use std::{
    io::{Read, Write},
    process::{Command, Stdio},
};

pub trait Clipboard {
    fn copy(text: &str);
    fn paste() -> String;
}

macro_rules! c {
    ($p:ident $($args:ident)+) => {
        Command::new(stringify!($p)).args([$(stringify!($args),)+])
    };
    ($p:literal) => {
        Command::new($p)
    };
    ($p:literal $($args:literal)+) => {
        Command::new($p).args([$($args,)+])

    }
}

trait Eat {
    fn eat(&mut self) -> String;
}

impl Eat for Command {
    fn eat(&mut self) -> String {
        let mut s = String::new();
        self.stdout(Stdio::piped())
            .spawn()
            .expect("spawn ok")
            .stdout
            .take()
            .unwrap()
            .read_to_string(&mut s)
            .expect("read ok");
        s
    }
}

trait Put {
    fn put(&mut self, s: impl AsRef<[u8]>);
}

impl Put for Command {
    fn put(&mut self, s: impl AsRef<[u8]>) {
        let mut ch = self.stdin(Stdio::piped()).spawn().expect("spawn ok");
        ch.stdin
            .take()
            .unwrap()
            .write_all(s.as_ref())
            .expect("write ok");
        ch.wait().expect("proc ok");
    }
}

#[cfg(target_os = "macos")]
pub struct PbCopy {}
#[cfg(target_os = "macos")]
impl Clipboard for PbCopy {
    fn copy(text: &str) {
        c!(pbcopy w).put(text)
    }

    fn paste() -> String {
        c!(pbcopy r).eat()
    }
}

pub struct XClip {}
impl Clipboard for XClip {
    fn copy(text: &str) {
        c!("xclip" "-selection" "c").put(text);
    }

    fn paste() -> String {
        c!("xclip" "-selection" "c" "-o") // xcclip is complainy
            .stderr(Stdio::null())
            .stdout(Stdio::null())
            .eat()
    }
}

pub struct XSel {}
impl Clipboard for XSel {
    fn copy(text: &str) {
        c!("xsel" "-b" "-i").put(text);
    }

    fn paste() -> String {
        c!("xsel" "-b" "-o").eat()
    }
}

struct Wayland {}
impl Clipboard for Wayland {
    fn copy(text: &str) {
        match text {
            "" => assert!(
                c!("wl-copy" "-p" "--clear").status().unwrap().success(),
                "wl-copy fail"
            ),
            s => c!("wl-copy" "-p").put(s),
        }
    }

    fn paste() -> String {
        c!("wl-paste" "-n" "-p").eat()
    }
}

struct Klipper {}
impl Clipboard for Klipper {
    fn copy(text: &str) {
        c!("qdbus" "org.kde.klipper" "/klipper" "setClipboardContents").arg(text);
    }

    fn paste() -> String {
        let mut s = c!("qdbus" "org.kde.klipper" "/klipper" "getClipboardContents").eat();
        assert!(s.ends_with('\n'));
        s.truncate(s.len() - 1);
        s
    }
}

#[cfg(target_family = "windows")]
struct Windows {}
#[cfg(target_family = "windows")]
impl Clipboard for Windows {
    fn copy(text: &str) {
        clipboard_win::set_clipboard_string(text).expect("set clip ok")
    }

    fn paste() -> String {
        clipboard_win::get_clipboard_string().expect("get clip ok")
    }
}

struct Wsl {}

impl Clipboard for Wsl {
    fn copy(text: &str) {
        c!("clip.exe").put(text);
    }

    fn paste() -> String {
        let mut s = c!("powershell.exe" "-noprofile" "-command" "Get-Clipboard").eat();
        s.truncate(s.len() - 2); // \r\n
        s
    }
}

pub type Board = (for<'a> fn(&'a str), fn() -> String);

fn get<T: Clipboard>() -> Board {
    (T::copy, T::paste)
}

fn has(c: &str) -> bool {
    c!("which")
        .arg(c)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .expect("ok")
        .success()
}

fn wsl() -> bool {
    if let Ok(s) = std::fs::read_to_string("/proc/version") {
        if s.to_lowercase().contains("microsoft") {
            return true;
        }
    }
    false
}

pub fn provide() -> Board {
    #[cfg(target_family = "windows")]
    return get::<Windows>();
    #[cfg(target_os = "macos")]
    return get::<PbCopy>();

    if wsl() {
        return get::<Wsl>();
    }
    assert!(std::env::var("DISPLAY").is_ok(), "no clipboard available");
    if std::env::var("WAYLAND_DISPLAY").is_ok() && has("wl-copy") {
        get::<Wayland>()
    } else if has("xsel") {
        get::<XSel>()
    } else if has("xclip") {
        get::<XClip>()
    } else if has("klipper") && has("qdbus") {
        get::<Klipper>()
    } else {
        panic!("no clipboard available");
    }
}

#[test]
fn test() {
    macro_rules! test {
        ($clipboard:ty) => {
            <$clipboard>::copy("text");
            assert_eq!(<$clipboard>::paste(), "text");
            <$clipboard>::copy("");
        };
    }
    #[cfg(target_os = "macos")]
    test!(PbCopy);
    #[cfg(target_os = "linux")]
    test!(XClip);
    #[cfg(target_os = "linux")]
    test!(XSel);
    #[cfg(target_os = "linux")]
    if std::env::var("WAYLAND_DISPLAY").is_ok() {
        test!(Wayland);
    }
    #[cfg(target_os = "linux")]
    test!(Klipper);
    #[cfg(target_family = "windows")]
    test!(Windows);
    if wsl() {
        #[cfg(target_os = "linux")]
        test!(Wsl);
    }
}
