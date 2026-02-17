use super::state::Section;

pub fn section_label(section: Section) -> &'static str {
    match section {
        Section::NowPlaying => "now playing",
        Section::Playlists => "playlists",
        Section::Queue => "queue",
        Section::Liked => "liked",
        Section::Search => "search",
        Section::Devices => "devices",
        Section::PlaylistTracks => "playlist tracks",
        Section::Setup => "setup",
        Section::Help => "help",
        Section::Auth => "auth",
    }
}

pub fn paginate(total: usize, selected: usize, page_size: usize) -> (usize, usize, usize, usize) {
    if total == 0 {
        return (0, 0, 0, 0);
    }
    let page_size = if page_size == 0 { total } else { page_size };
    let selected = selected.min(total.saturating_sub(1));
    let start = if selected >= page_size {
        selected - page_size + 1
    } else {
        0
    };
    let end = (start + page_size).min(total);
    let pages = total.div_ceil(page_size);
    let page = (selected / page_size) + 1;
    (start, end, page, pages)
}

pub fn list_page_size(height: u16) -> usize {
    if height <= 6 {
        return 5;
    }
    let size = height.saturating_sub(6);
    if size < 5 {
        5
    } else {
        size as usize
    }
}

pub fn truncate(s: &str, max_len: usize) -> String {
    if max_len == 0 {
        return String::new();
    }
    if max_len == 1 {
        return "…".to_string();
    }
    if s.chars().count() <= max_len {
        return s.to_string();
    }

    let mut out = String::with_capacity(max_len);
    out.extend(s.chars().take(max_len - 1));
    out.push('…');
    out
}

pub fn format_time(seconds: f64) -> String {
    let m = (seconds as u64) / 60;
    let s = (seconds as u64) % 60;
    format!("{}:{:02}", m, s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paginate_uses_selected_page() {
        let (_start, _end, page, pages) = paginate(20, 16, 5);
        assert_eq!(page, 4);
        assert_eq!(pages, 4);
    }

    #[test]
    fn list_page_size_minimum() {
        assert_eq!(list_page_size(0), 5);
        assert_eq!(list_page_size(4), 5);
    }

    #[test]
    fn truncate_handles_multibyte_characters() {
        let title = "2000'S NAIJA HITS 🇳🇬🇳🇬";
        let got = truncate(title, 21);
        assert_eq!(got, "2000'S NAIJA HITS 🇳🇬…");
    }
}
