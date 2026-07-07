#[derive(Clone, Default)]
pub struct AudioTags {
    pub title: String,
    pub artist: String,
    pub album: String,
    pub year: String,
    pub genre: String,
    pub track: String,
    pub bpm: String,
    pub key: String,
}

pub fn read_audio_tags(path: &str) -> AudioTags {
    use lofty::prelude::{Accessor, ItemKey, TaggedFileExt};
    let mut t = AudioTags::default();
    let Ok(tagged) = lofty::read_from_path(path) else {
        return t;
    };
    let Some(tag) = tagged.primary_tag().or_else(|| tagged.first_tag()) else {
        return t;
    };
    t.title = tag.title().map(|c| c.to_string()).unwrap_or_default();
    t.artist = tag.artist().map(|c| c.to_string()).unwrap_or_default();
    t.album = tag.album().map(|c| c.to_string()).unwrap_or_default();
    t.year = tag
        .get_string(ItemKey::Year)
        .or_else(|| tag.get_string(ItemKey::RecordingDate))
        .unwrap_or_default()
        .to_string();
    t.genre = tag.genre().map(|c| c.to_string()).unwrap_or_default();
    t.track = tag.track().map(|n| n.to_string()).unwrap_or_default();
    t.bpm = tag
        .get_string(ItemKey::IntegerBpm)
        .or_else(|| tag.get_string(ItemKey::Bpm))
        .unwrap_or_default()
        .to_string();
    t.key = tag.get_string(ItemKey::InitialKey).unwrap_or_default().to_string();
    t
}

pub fn write_audio_tags(path: &str, t: &AudioTags) -> Result<(), Box<dyn std::error::Error>> {
    use lofty::config::WriteOptions;
    use lofty::prelude::{Accessor, ItemKey, TagExt, TaggedFileExt};
    use lofty::tag::Tag;

    let mut tagged = lofty::read_from_path(path)?;
    let tag_type = tagged.primary_tag_type();
    if tagged.primary_tag_mut().is_none() {
        tagged.insert_tag(Tag::new(tag_type));
    }
    let tag = tagged
        .primary_tag_mut()
        .ok_or("não foi possível criar a tag")?;

    if t.title.trim().is_empty() {
        tag.remove_title();
    } else {
        tag.set_title(t.title.clone());
    }
    if t.artist.trim().is_empty() {
        tag.remove_artist();
    } else {
        tag.set_artist(t.artist.clone());
    }
    if t.album.trim().is_empty() {
        tag.remove_album();
    } else {
        tag.set_album(t.album.clone());
    }
    if !t.year.trim().is_empty() {
        tag.insert_text(ItemKey::Year, t.year.trim().to_string());
    }
    if t.genre.trim().is_empty() {
        tag.remove_genre();
    } else {
        tag.set_genre(t.genre.clone());
    }
    if let Ok(n) = t.track.trim().parse::<u32>() {
        tag.set_track(n);
    }
    if !t.bpm.trim().is_empty() {
        tag.insert_text(ItemKey::IntegerBpm, t.bpm.trim().to_string());
    }
    if !t.key.trim().is_empty() {
        tag.insert_text(ItemKey::InitialKey, t.key.trim().to_string());
    }

    tag.save_to_path(path, WriteOptions::default())?;
    Ok(())
}
