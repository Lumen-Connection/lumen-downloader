use super::models::ThumbImage;

pub async fn download_thumbnail(url: &str) -> Option<ThumbImage> {
    let bytes = reqwest::get(url).await.ok()?.bytes().await.ok()?;
    let img = image::load_from_memory(&bytes).ok()?;
    let img = if img.width() > 360 {
        img.resize(
            360,
            (img.height() * 360 / img.width().max(1)).max(1),
            image::imageops::FilterType::Triangle,
        )
    } else {
        img
    };
    let rgba = img.to_rgba8();
    let (w, h) = rgba.dimensions();
    Some(ThumbImage {
        width: w as usize,
        height: h as usize,
        rgba: rgba.into_raw(),
    })
}
