//! ASCII QR Code renderer for terminal display of Solana Pay URLs.

use qrcodegen::{QrCode, QrCodeEcc};

/// Renders a string as an ASCII QR code suitable for terminal scanning.
pub fn render_qr_ascii(text: &str) -> String {
    let qr = match QrCode::encode_text(text, QrCodeEcc::Low) {
        Ok(q) => q,
        Err(_) => return format!("[QR Code generation failed for: {text}]"),
    };

    let mut out = String::new();
    let border = 1;
    let size = qr.size();

    // Top quiet zone
    for _ in 0..(size + border * 2) {
        out.push_str("██");
    }
    out.push('\n');

    for y in 0..size {
        for _ in 0..border {
            out.push_str("██");
        }
        for x in 0..size {
            if qr.get_module(x, y) {
                out.push_str("  "); // Black module
            } else {
                out.push_str("██"); // White module
            }
        }
        for _ in 0..border {
            out.push_str("██");
        }
        out.push('\n');
    }

    // Bottom quiet zone
    for _ in 0..(size + border * 2) {
        out.push_str("██");
    }
    out.push('\n');

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_non_empty_qr_ascii() {
        let qr = render_qr_ascii("solana:7Xw7K1kPzSjW7Y2qM3nK4pL5rQ8sT9uV8wX1yZ2a3b4c?amount=10");
        assert!(!qr.is_empty());
        assert!(qr.contains("██"));
    }
}
