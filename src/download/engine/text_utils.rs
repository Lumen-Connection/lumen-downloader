pub fn sanitize_filename(name: &str) -> String {
    let invalid_chars = ['<', '>', ':', '"', '/', '\\', '|', '?', '*', '\0'];
    let mut sanitized: String = name
        .chars()
        .filter(|c| !invalid_chars.contains(c) && !c.is_control())
        .collect();
    sanitized.truncate(200);
    if sanitized.trim().is_empty() {
        sanitized = "download".to_string();
    }
    sanitized
}

pub fn smart_clean_name(title: &str) -> String {
    const JUNK: &[&str] = &[
        "official", "oficial", "video", "vídeo", "audio", "áudio", "lyric", "letra",
        "lyrics", "hd", "4k", "8k", "mv", "m/v", "clipe", "visualizer", "remaster",
        "remastered", "explicit", "full album", "hq",
    ];
    let chars: Vec<char> = title.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < chars.len() {
        let close = match chars[i] {
            '[' => Some(']'),
            '(' => Some(')'),
            '{' => Some('}'),
            _ => None,
        };
        if let Some(cl) = close {
            if let Some(j) = (i + 1..chars.len()).find(|&k| chars[k] == cl) {
                let inner: String = chars[i + 1..j].iter().collect::<String>().to_lowercase();
                if JUNK.iter().any(|k| inner.contains(k)) {
                    i = j + 1;
                    continue;
                }
            }
        }
        out.push(chars[i]);
        i += 1;
    }
    let out = out.replace(" - Topic", "").replace("- Topic", "");
    let collapsed = out.split_whitespace().collect::<Vec<_>>().join(" ");
    let trimmed = collapsed
        .trim()
        .trim_matches(|c| c == '-' || c == '|' || c == '·' || c == '_')
        .trim()
        .to_string();
    if trimmed.is_empty() {
        title.trim().to_string()
    } else {
        trimmed
    }
}

pub fn apply_template(template: &str, title: &str, channel: &str) -> String {
    let mut s = template.replace("%(title)s", title);
    s = s.replace("%(uploader)s", channel).replace("%(channel)s", channel);
    if s.trim().is_empty() {
        s = title.to_string();
    }
    s
}
