pub const LED_COUNT: usize = 30;
pub const FRAME_SIZE: usize = 5 + LED_COUNT * 3;
pub const PROBE_SIZE: usize = 16;

const HEADER: [u8; 5] = [0x00, 0x53, 0x02, 0x1E, 0x00];

pub type Rgb = (u8, u8, u8);

pub fn probe_is_valid(reply: &[u8; PROBE_SIZE]) -> bool {
    reply[0..3] == *b"ISK" && reply[13..15] == *b"AP"
}

pub fn solid(color: Rgb) -> [u8; FRAME_SIZE] {
    from_pixels(&[color; LED_COUNT])
}

pub fn from_pixels(pixels: &[Rgb; LED_COUNT]) -> [u8; FRAME_SIZE] {
    let mut frame = [0u8; FRAME_SIZE];
    frame[..HEADER.len()].copy_from_slice(&HEADER);

    for (i, &(r, g, b)) in pixels.iter().enumerate() {
        // Recovered from MAXSUN AacHal: wire order is G, R, B.
        let o = HEADER.len() + i * 3;
        frame[o] = g;
        frame[o + 1] = r;
        frame[o + 2] = b;
    }

    frame
}

pub fn gradient(start: Rgb, end: Rgb) -> [u8; FRAME_SIZE] {
    let mut pixels = [(0u8, 0u8, 0u8); LED_COUNT];
    let den = (LED_COUNT - 1) as u32;

    for (i, pixel) in pixels.iter_mut().enumerate() {
        let n = i as u32;
        *pixel = (
            lerp(start.0, end.0, n, den),
            lerp(start.1, end.1, n, den),
            lerp(start.2, end.2, n, den),
        );
    }

    from_pixels(&pixels)
}

pub fn static_rainbow() -> [u8; FRAME_SIZE] {
    let mut pixels = [(0u8, 0u8, 0u8); LED_COUNT];
    for (i, pixel) in pixels.iter_mut().enumerate() {
        let hue = ((i * 256) / LED_COUNT) as u8;
        *pixel = wheel(hue);
    }
    from_pixels(&pixels)
}

pub fn parse_frame(values: &[String]) -> Result<[u8; FRAME_SIZE], String> {
    if values.len() != LED_COUNT {
        return Err(format!("frame requires exactly {LED_COUNT} RRGGBB colors"));
    }

    let mut pixels = [(0u8, 0u8, 0u8); LED_COUNT];
    for (dst, src) in pixels.iter_mut().zip(values) {
        *dst = parse_rgb(src)?;
    }
    Ok(from_pixels(&pixels))
}

pub fn parse_rgb(value: &str) -> Result<Rgb, String> {
    let s = value.trim().trim_start_matches('#');
    if s.len() != 6 || !s.bytes().all(|c| c.is_ascii_hexdigit()) {
        return Err(format!("invalid color '{value}'; expected RRGGBB, e.g. FF8000"));
    }

    Ok((
        u8::from_str_radix(&s[0..2], 16).map_err(|_| "invalid red component")?,
        u8::from_str_radix(&s[2..4], 16).map_err(|_| "invalid green component")?,
        u8::from_str_radix(&s[4..6], 16).map_err(|_| "invalid blue component")?,
    ))
}

fn lerp(a: u8, b: u8, n: u32, den: u32) -> u8 {
    let a = a as i32;
    let b = b as i32;
    (a + ((b - a) * n as i32 / den as i32)) as u8
}

fn wheel(h: u8) -> Rgb {
    // Integer HSV(h, 1, 1), six sectors.
    let x = h as u16 * 6;
    let sector = (x >> 8) as u8;
    let f = (x & 0xFF) as u8;
    let q = 255u8.saturating_sub(f);

    match sector {
        0 => (255, f, 0),
        1 => (q, 255, 0),
        2 => (0, 255, f),
        3 => (0, q, 255),
        4 => (f, 0, 255),
        _ => (255, 0, q),
    }
}
