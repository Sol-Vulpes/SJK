//! A pack's sound files: a real WAV (RIFF, a format the game plays) or MP3 (a frame
//! header after any ID3 tag), read from their headers only.

/// Why `bytes` is not a sound of `kind` (`wav` or `mp3`) the game can play.
pub(crate) fn check(kind: &str, bytes: &[u8]) -> Result<(), &'static str> {
    match kind {
        "wav" => wav(bytes),
        _ => mp3(bytes),
    }
}

fn wav(bytes: &[u8]) -> Result<(), &'static str> {
    if bytes.len() < 12 || &bytes[..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Err("not a WAV file");
    }
    let mut at = 12;
    let mut format = None;
    let mut data = false;
    while at + 8 <= bytes.len() {
        let id = &bytes[at..at + 4];
        let size = u32::from_le_bytes(bytes[at + 4..at + 8].try_into().expect("4 bytes")) as usize;
        let body = at + 8;
        if id == b"fmt " {
            let fmt = bytes
                .get(body..body + 16)
                .ok_or("its fmt chunk is cut short")?;
            let tag = u16::from_le_bytes([fmt[0], fmt[1]]);
            let channels = u16::from_le_bytes([fmt[2], fmt[3]]);
            let rate = u32::from_le_bytes(fmt[4..8].try_into().expect("4 bytes"));
            format = Some((tag, channels, rate));
        } else if id == b"data" {
            data = true;
            break;
        }
        // Chunks are padded to an even size.
        at = body.saturating_add(size).saturating_add(size & 1);
    }
    let (tag, channels, rate) = format.ok_or("a WAV without its format")?;
    // PCM, Microsoft and IMA ADPCM, MP3 in a WAV: what the game's decoders take.
    if !matches!(tag, 1 | 2 | 0x11 | 0x55) {
        return Err("a WAV in a format the game does not play (use 16-bit PCM)");
    }
    if !(1..=2).contains(&channels) || !(4000..=48000).contains(&rate) {
        return Err("a WAV with more than 2 channels or an odd sample rate");
    }
    if !data {
        return Err("a WAV without sound data");
    }
    Ok(())
}

fn mp3(bytes: &[u8]) -> Result<(), &'static str> {
    let mut at = 0;
    if bytes.len() >= 10 && &bytes[..3] == b"ID3" {
        // An ID3v2 tag: its size is 4 bytes of 7 bits each, after a 10-byte header.
        let size = bytes[6..10]
            .iter()
            .fold(0usize, |size, &b| (size << 7) | usize::from(b & 0x7f));
        at = 10 + size;
    }
    let header = bytes.get(at..at + 4).ok_or("an MP3 without sound")?;
    let sync = header[0] == 0xff && header[1] & 0xe0 == 0xe0;
    let layer = (header[1] >> 1) & 0b11;
    let bitrate = header[2] >> 4;
    let rate = (header[2] >> 2) & 0b11;
    if !sync || layer == 0 || bitrate == 0b1111 || rate == 0b11 {
        return Err("not an MP3 file");
    }
    Ok(())
}

#[cfg(test)]
pub(crate) fn test_wav() -> Vec<u8> {
    let mut wav = b"RIFF\0\0\0\0WAVEfmt \x10\0\0\0".to_vec();
    wav.extend(1u16.to_le_bytes());
    wav.extend(1u16.to_le_bytes());
    wav.extend(22050u32.to_le_bytes());
    wav.extend(44100u32.to_le_bytes());
    wav.extend(2u16.to_le_bytes());
    wav.extend(16u16.to_le_bytes());
    wav.extend(b"data\x04\0\0\0\0\0\0\0");
    wav
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn real_sounds_pass_and_others_do_not() {
        assert_eq!(check("wav", &test_wav()), Ok(()));
        let mut stereo_float = test_wav();
        stereo_float[20] = 3;
        assert!(check("wav", &stereo_float).is_err());
        assert!(check("wav", b"RIFF\0\0\0\0WAVE").is_err());
        assert!(check("wav", b"MZ not a sound").is_err());
        assert_eq!(check("mp3", &[0xff, 0xfb, 0x90, 0x44, 0, 0]), Ok(()));
        let mut tagged = b"ID3\x04\0\0\0\0\0\x02xx".to_vec();
        tagged.extend([0xff, 0xfb, 0x90, 0x44]);
        assert_eq!(check("mp3", &tagged), Ok(()));
        assert!(check("mp3", b"<html>").is_err());
    }
}
