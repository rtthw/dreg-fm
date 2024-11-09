//! Widgets



use dreg::prelude::*;



pub struct Block {
    pub style: Style,
}

impl Block {
    pub fn new(style: Style) -> Self {
        Self {
            style,
        }
    }
    
    pub fn render(self, area: Rect, buf: &mut Buffer) {
        for y in area.top()..area.bottom() {
            buf.get_mut(area.x, y).set_char('│').set_style(self.style);
            buf.get_mut(area.right().saturating_sub(1), y).set_char('│').set_style(self.style);
        }
        for x in area.left()..area.right() {
            buf.get_mut(x, area.y).set_char('─').set_style(self.style);
            buf.get_mut(x, area.bottom().saturating_sub(1)).set_char('─').set_style(self.style);
        }
        buf.get_mut(area.x, area.y).set_char('┌').set_style(self.style);
        buf.get_mut(area.right().saturating_sub(1), area.y).set_char('┐').set_style(self.style);
        buf.get_mut(area.right().saturating_sub(1), area.bottom().saturating_sub(1)).set_char('┘').set_style(self.style);
        buf.get_mut(area.x, area.bottom().saturating_sub(1)).set_char('└').set_style(self.style);
    }
}
