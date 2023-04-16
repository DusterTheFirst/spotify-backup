use cansi::{Color, Intensity};
use dioxus::prelude::*;

#[inline_props]
pub fn preformatted_ansi(cx: Scope<'_, Props>, ansi_text: String) -> Element<'_> {
    let slices = cansi::v3::categorise_text(ansi_text);

    cx.render(rsx! {
        pre {
            style: "background-color: #181818; color: #cccccc;",
            code {
                slices.into_iter().map(|slice| {
                    // TODO: more styling
                    let color = slice.fg.map(terminal_color_to_hex).unwrap_or("inherit");
                    let font_weight = slice.intensity.map(intensity_to_css).unwrap_or("inherit");

                    rsx! {
                        span {
                            style: "color: {color}; font-weight: {font_weight}",
                            slice.text
                        }
                    }
                })
            }
        }
    })
}

fn terminal_color_to_hex(color: Color) -> &'static str {
    // Stolen from: https://github.com/microsoft/vscode/blob/1e774371f2ca5f6618b4f40fdc72ce7518443014/src/vs/workbench/contrib/terminal/common/terminalColorRegistry.ts#L161
    match color {
        Color::Black => "#000000",         // Light: 000000
        Color::Red => "#cd3131",           // Light: cd3131
        Color::Green => "#0dbc79",         // Light: 00bc00
        Color::Yellow => "#e5e510",        // Light: 949800
        Color::Blue => "#2472c8",          // Light: 0451a5
        Color::Magenta => "#bc3fbc",       // Light: bc05bc
        Color::Cyan => "#11a8cd",          // Light: 0598bc
        Color::White => "#e5e5e5",         // Light: 555555
        Color::BrightBlack => "#666666",   // Light: 666666
        Color::BrightRed => "#f14c4c",     // Light: cd3131
        Color::BrightGreen => "#23d18b",   // Light: 14ce14
        Color::BrightYellow => "#f5f543",  // Light: b5ba00
        Color::BrightBlue => "#3b8eea",    // Light: 0451a5
        Color::BrightMagenta => "#d670d6", // Light: bc05bc
        Color::BrightCyan => "#0598bc",    // Light: 29b8db
        Color::BrightWhite => "#a5a5a5",   // Light: e5e5e5
    }
}

fn intensity_to_css(intensity: Intensity) -> &'static str {
    match intensity {
        Intensity::Normal => "normal",
        Intensity::Bold => "bold",
        Intensity::Faint => "thin",
    }
}
