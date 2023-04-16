use core::fmt::{Display, Write};

use anes::parser::Parser;
use dioxus::prelude::*;

#[inline_props]
pub fn preformatted_ansi<'a>(cx: Scope<'a>, text: &'a dyn Display) -> Element<'a> {
    let mut writer = Writer {
        parser: anes::parser::Parser::default(),
    };

    writer.write_fmt(format_args!("{}", text)).throw(cx)?;

    cx.render(rsx! {
        for sequence in writer.parser {
            "{sequence:?}"
        }
    })
}

struct Writer {
    parser: Parser,
}

impl core::fmt::Write for Writer {
    fn write_char(&mut self, c: char) -> core::fmt::Result {
        self.write_str(c.encode_utf8(&mut [0; 4]))
    }

    fn write_fmt(mut self: &mut Self, args: core::fmt::Arguments<'_>) -> core::fmt::Result {
        core::fmt::write(&mut self, args)
    }

    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        self.parser.advance(s.as_bytes(), true);

        Ok(())
    }
}
