pub fn get_font_properties(font: &str) -> Option<(String, f32)> {
    let font_vec = font.split_whitespace().collect::<Vec<_>>();
    font_vec.split_last().map(|(size, name)| {
        let font_name = name.join(" ");
        let font_size = size.parse::<f32>().unwrap();
        (font_name, font_size)
    })
}
