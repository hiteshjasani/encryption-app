use iced::Color;

pub fn offset_color(in_color: Color, offset: u8) -> Color {
  let mut ca = in_color.into_rgba8();
  ca[0] += offset % 255;
  ca[1] += offset % 255;
  ca[2] += offset % 255;
  Color::from_rgba8(ca[0], ca[1], ca[2], ca[3].into())
}
