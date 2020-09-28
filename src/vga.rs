//! VGA driver
// 
//  Copyright 2020 Mike Leany
// 
//  Licensed under the Apache License, Version 2.0 (the "License");
//  you may not use this file except in compliance with the License.
//  You may obtain a copy of the License at
// 
//      <http://www.apache.org/licenses/LICENSE-2.0>
// 
//  Unless required by applicable law or agreed to in writing, software
//  distributed under the License is distributed on an "AS IS" BASIS,
//  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
//  See the License for the specific language governing permissions and
//  limitations under the License.
///////////////////////////////////////////////////////////////////////////////////////////////////
use core::fmt::{self, Write};
use core::convert::TryFrom;
use core::ops;
use lazy_static::lazy_static;
use spin::Mutex;

/// A struct to represent the VGA console.
#[derive(Debug, Clone)]
pub struct Console(&'static Mutex<ConsoleData>);

impl Console {
    /// The number of `Glyphs` that can be displayed in one line on the screen.
    pub const WIDTH: usize = 80;
    /// The number of lines on the screen.
    pub const HEIGHT: usize = 25;
    /// The number of lines in the buffer.
    pub const BUFFER_LINES: usize = Self::HEIGHT + 1;
    /// The number of `Glyphs` that will fit between tab stops.
    pub const TAB_WIDTH: usize = 8;
}

impl Write for Console {
    /// Writes a string to the `Console`.
    ///
    /// The following characters are given special treatment.
    ///
    /// - The newline (`'\n'`) advances to the beginning of the next line. 
    /// - The carriage return (`'\r'`) is ignored.
    /// - The tab (`'\t'`) advances to the next tab stop.
    ///
    /// All other characters in the string are first converted to `Glyph`s, then written to the
    /// screen. Any characters that cannot be converted are replaced with `Glyph::REPLACEMENT`.
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.0.lock().write_str(s);

        Ok(())
    }
}

/// The data for a `Console`.
#[derive(Debug)]
struct ConsoleData {
    video_mem: *mut [ColoredGlyph; Console::WIDTH],
    buffer: [[ColoredGlyph; Console::WIDTH]; Console::BUFFER_LINES],
    colors: Colors,
    loc: Location,
}

impl ConsoleData {
    fn write_str(&mut self, s: &str) {
        let mut new_loc = self.loc;

        for c in s.chars() {
            new_loc = match c {
                '\n' => { new_loc.next_line() },
                '\t' => { new_loc.next_tab() },
                '\r' => { new_loc },
                _ => {
                    // write the glyph
                    let buf_line = new_loc.line() % Console::BUFFER_LINES;
                    self.buffer[buf_line][new_loc.col()] = ColoredGlyph {
                        glyph: Glyph::try_from(c).unwrap_or(Glyph::REPLACEMENT),
                        colors: self.colors,
                    };

                    new_loc + 1
                },
            };

            if new_loc.line() > self.loc.line() {
                self.buffer[new_loc.line() % Console::BUFFER_LINES]
                    = [ColoredGlyph::null(self.colors); Console::WIDTH];
                self.scroll_and_flush(new_loc);
            }
        }
        self.flush(new_loc);
    }

    fn scroll_and_flush(&mut self, new_loc: Location) {
        let top_line = (new_loc.line() + 1).saturating_sub(Console::HEIGHT);

        for (scr_line, line) in (top_line..).take(Console::HEIGHT).enumerate() {
            let buf_line = line % Console::BUFFER_LINES;

            unsafe {
                // SAFETY: sound because `self.video_mem` should always point to a location of
                // Console::HEIGHT lines that we have access to, and `scr_line` is always less than
                // `Console::HEIGHT`. Also, access to `ConsoleData`, which is private, is synchronized
                // using a Mutex, which prevents data races.
                self.video_mem.add(scr_line).write_volatile(self.buffer[buf_line]);
            }
        }

        self.loc = new_loc;
    }

    fn flush(&mut self, new_loc: Location) {
        let scr_line = core::cmp::min(new_loc.line(), Console::HEIGHT - 1);
        let buf_line = new_loc.line() % Console::BUFFER_LINES;

        unsafe {
            // SAFETY: sound because `self.video_mem` should always point to a location of
            // Console::HEIGHT lines that we have access to, and `scr_line` is always less than
            // `Console::HEIGHT`. Also, access to `ConsoleData`, which is private, is synchronized
            // using a Mutex, which prevents data races.
            self.video_mem.add(scr_line).write_volatile(self.buffer[buf_line]);
        }

        self.loc = new_loc;
    }
}

// SAFETY: sound because only one instance of `ConsoleData` is ever created, and its pointer
// `video_mem` is never accessed outside of `ConsoleData`. Also, access to the only instance of
// `ConsoleData` is synchronized using a `Mutex`.
unsafe impl Send for ConsoleData { }

/// Returns a handle to the VGA `Console`. Writes to the `Console` are synchronized and are thus
/// thread safe.
pub fn console() -> Console {
    const VIDEO_MEM_ADDR: u64 = 0xb8000;
    static CONSOLE: Mutex<ConsoleData> = Mutex::new(ConsoleData {
        video_mem: VIDEO_MEM_ADDR as *mut [ColoredGlyph; Console::WIDTH],
        buffer: [[ColoredGlyph::null(Colors::new()); Console::WIDTH]; Console::BUFFER_LINES],
        colors: Colors::new(),
        loc: Location::new(),
    });
    static INIT: spin::Once<()> = spin::Once::new();

    INIT.call_once(|| {
        // clear the screen
        CONSOLE.lock().scroll_and_flush(Location::default());

        // hide the cursor
        // TODO
    });

    Console(&CONSOLE)
}

/// Helper function for the `print!` macro.
#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    console().write_fmt(args).expect("INFALLIBLE");
}

/// Prints to the screen.
///
/// The following characters are given special treatment.
///
/// - The newline (`'\n'`) moves the cursor to the beginning of the next line.
/// - The carriage return (`'\r'`) moves the cursor to the beginning of the current line.
/// - The tab (`'\t'`) moves the cursor to the next tab stop.
///
/// All other characters in the string are first converted to `Glyph`s, then written to the
/// screen. Any characters that cannot be converted are replaced with `Glyph::REPLACEMENT`.
#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::vga::_print(core::format_args!($($arg)*)));
}

/// Prints to the screen, with a newline. Otherwise the same as [`print`](macro.print.html).
#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)+) => ({
        $crate::print!("{}\n", core::format_args!($($arg)+));
    })
}

/// A glyph, corresponding to [Code page 437](https://en.wikipedia.org/wiki/Code_page_437) which can
/// be written to the screen.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Default)]
#[repr(transparent)]
pub struct Glyph(u8);

impl Glyph {
    /// A `Glyph` ('■') to replace `char`s which cannot be translated to a `Glyph`.
    pub const REPLACEMENT: Glyph = Glyph(0xfe);

    /// Table to translate `Glyph`s to `char`s.
    const CHARS: [char; 256] = [
        '\0','☺', '☻', '♥', '♦', '♣', '♠', '•', '◘', '○', '◙', '♂', '♀', '♪', '♫', '☼',
        '►', '◄', '↕', '‼', '¶', '§', '▬', '↨', '↑', '↓', '→', '←', '∟', '↔', '▲', '▼',
        ' ', '!', '"', '#', '$', '%', '&', '\'','(', ')', '*', '+', ',', '-', '.', '/',
        '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', ':', ';', '<', '=', '>', '?',
        '@', 'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M', 'N', 'O',
        'P', 'Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y', 'Z', '[', '\\',']', '^', '_',
        '`', 'a', 'b', 'c', 'd', 'e', 'f', 'g', 'h', 'i', 'j', 'k', 'l', 'm', 'n', 'o',
        'p', 'q', 'r', 's', 't', 'u', 'v', 'w', 'x', 'y', 'z', '{', '|', '}', '~', '⌂',
        'Ç', 'ü', 'é', 'â', 'ä', 'à', 'å', 'ç', 'ê', 'ë', 'è', 'ï', 'î', 'ì', 'Ä', 'Å',
        'É', 'æ', 'Æ', 'ô', 'ö', 'ò', 'û', 'ù', 'ÿ', 'Ö', 'Ü', '¢', '£', '¥', '₧', 'ƒ',
        'á', 'í', 'ó', 'ú', 'ñ', 'Ñ', 'ª', 'º', '¿', '⌐', '¬', '½', '¼', '¡', '«', '»',
        '░', '▒', '▓', '│', '┤', '╡', '╢', '╖', '╕', '╣', '║', '╗', '╝', '╜', '╛', '┐',
        '└', '┴', '┬', '├', '─', '┼', '╞', '╟', '╚', '╔', '╩', '╦', '╠', '═', '╬', '╧',
        '╨', '╤', '╥', '╙', '╘', '╒', '╓', '╫', '╪', '┘', '┌', '█', '▄', '▌', '▐', '▀',
        'α', 'ß', 'Γ', 'π', 'Σ', 'σ', 'µ', 'τ', 'Φ', 'Θ', 'Ω', 'δ', '∞', 'φ', 'ε', '∩',
        '≡', '±', '≥', '≤', '⌠', '⌡', '÷', '≈', '°', '∙', '·', '√', 'ⁿ', '²', '■', '\u{00a0}',
    ];

    /// Returns the `Glyph` by index. This is not to be confused with the ASCII value. The index
    /// for each `Glyph` comes from [Code page 437](https://en.wikipedia.org/wiki/Code_page_437).
    pub const fn from_index(index: u8) -> Glyph {
        Self(index)
    }
}

impl From<Glyph> for char {
    /// Returns the `char` represented by the `Glyph`.
    fn from(glyph: Glyph) -> char {
        Glyph::CHARS[glyph.0 as usize]
    }
}

impl TryFrom<char> for Glyph {
    type Error = TryFromCharError;

    /// Returns the `Glyph` that represents a `char`, if one is available.
    fn try_from(c: char) -> Result<Self, Self::Error> {
        const SIZE: usize = (0x100-0x7f) + 0x20;
        lazy_static! {
            static ref GLYPHS: [(char, Glyph); SIZE] = {
                let mut arr = [('\0', Glyph(0)); SIZE];

                // fill the array using `Glyph::CHARS`. We skip the printable ascii characters
                // (0x20 - 0x7e) since they match up exactly with Code page 437.
                for (i, &c) in Glyph::CHARS[0x00..0x20].iter().enumerate() {
                    arr[i] = (c, Glyph(i as u8));
                }
                for (i, &c) in Glyph::CHARS[0x7f..0x100].iter().enumerate() {
                    let glyph = i as u8 + 0x7f;
                    arr[i + 0x20] = (c, Glyph(glyph));
                }

                arr.sort_unstable_by_key(|(c, _)| *c);

                arr
            };
        }

        // printable ascii characters (0x20 - 0x7e)
        if c >= ' ' && c <= '~' {
            return Ok(Glyph(c as u8));
        }

        if let Ok(i) = GLYPHS.binary_search_by_key(&c, |(c, _)| *c) {
            return Ok(GLYPHS[i].1);
        }

        Err(TryFromCharError)
    }
}

/// An error that can result from an attempt to convert a `char` into a `Glyph`.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Default)]
pub struct TryFromCharError;

/// A color, which can be used for text and background colors.
#[allow(missing_docs)]
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[repr(u8)]
pub enum Color {
    Black         = 0x0,
    Blue          = 0x1,
    Green         = 0x2,
    Cyan          = 0x3,
    Red           = 0x4,
    Magenta       = 0x5,
    Brown         = 0x6,
    LightGray     = 0x7,
    DarkGray      = 0x8,
    LightBlue     = 0x9,
    LightGreen    = 0xa,
    LightCyan     = 0xb,
    LightRed      = 0xc,
    LightMagenta  = 0xd,
    Yellow        = 0xe,
    White         = 0xf,
}

/// The text and background colors to print a `Glyph`.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[repr(transparent)]
pub struct Colors(u8);

impl Colors {
    /// Create new `Colors` with the default settings, `LightGray` on `Black`.
    pub const fn new() -> Colors {
        Colors::new_from(Color::LightGray, Color::Black)
    }

    /// Create new `Colors` from text and background colors.
    pub const fn new_from(text: Color, background: Color) -> Colors {
        Colors((background as u8) << 4 | text as u8)
    }

    /// Sets the text color.
    pub fn set_text_color(&mut self, color: Color) {
        self.0 = (self.0 & 0xf0) | color as u8;
    }

    /// Sets the background color.
    pub fn set_background_color(&mut self, color: Color) {
        self.0 = (self.0 & 0x0f) | (color as u8) << 4;
    }

    /// Returns the text color.
    pub fn text(self) -> Color {
        // SAFETY: this is sound because the value is limited to the range 0x0 to 0xf, and all
        // discriminants in this range are defined in `Color`.
        unsafe { core::mem::transmute(self.0 & 0xf) }
    }

    /// Returns the background color.
    pub fn background(self) -> Color {
        // SAFETY: this is sound because the value is limited to the range 0x0 to 0xf, and all
        // discriminants in this range are defined in `Color`.
        unsafe { core::mem::transmute(self.0 >> 4) }
    }
}

impl Default for Colors {
    fn default() ->  Colors {
        Colors::new()
    }
}

/// The combination of a `Glyph` and `Colors`.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Default)]
#[repr(C)]
struct ColoredGlyph{
    glyph: Glyph,
    colors: Colors,
}

impl ColoredGlyph {
    const fn null(colors: Colors) -> ColoredGlyph {
        ColoredGlyph {
            glyph: Glyph(0),
            colors,
        }
    }
}

/// A location on the screen.
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Default)]
pub struct Location(usize);

impl Location {
    /// Returns the location of the first character of the first line.
    pub const fn new() -> Location {
        Self(0)
    }

    /// Returns the horizontal offset from the left-most column of the screen.
    pub fn col(self) -> usize {
        (self.0 % Console::WIDTH) as usize
    }

    /// Returns the total number of lines printed.
    pub fn line(self) -> usize {
        (self.0 / Console::WIDTH) as usize
    }

    /// Returns the `Location` of the next tab stop.
    pub fn next_tab(self) -> Location {
        Self((self.0 - self.0 % Console::TAB_WIDTH) + Console::TAB_WIDTH)
    }

    /// Returns the `Location` of the beginning of the next line.
    pub fn next_line(self) -> Location {
        Self((self.0 - self.0 % Console::WIDTH) + Console::WIDTH)
    }
}

impl ops::Add<usize> for Location {
    type Output = Self;

    fn add(self, n: usize) -> Self {
        Self(self.0 + n)
    }
}

impl ops::AddAssign<usize> for Location {
    fn add_assign(&mut self, n: usize) {
        self.0 += n;
    }
}
