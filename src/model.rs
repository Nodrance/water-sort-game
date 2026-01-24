use macroquad::prelude::*;

pub const FLUID_COLORS: [Color; 32] = [
    Color::new(1.0  , 0.0  , 0.0  , 1.0  ), //RED
    Color::new(0.0  , 0.0  , 1.0  , 1.0  ), //BLUE
    Color::new(1.0  , 1.0  , 0.0  , 1.0  ), //YELLOW
    Color::new(0.0  , 0.5  , 0.0  , 1.0  ), //GREEN
    Color::new(0.627, 0.125, 0.941, 1.0  ), //PURPLE
    Color::new(1.0  , 0.647, 0.0  , 1.0  ), //ORANGE
    Color::new(0.0  , 1.0  , 1.0  , 1.0  ), //CYAN
    Color::new(1.0  , 0.0  , 1.0  , 1.0  ), //MAGENTA
    Color::new(0.0  , 1.0  , 0.0  , 1.0  ), //LIME
    Color::new(1.0  , 0.752, 0.796, 1.0  ), //PINK
    Color::new(0.647, 0.164, 0.164, 1.0  ), //BROWN
    Color::new(0.0  , 0.0  , 0.5  , 1.0  ), //NAVY
    Color::new(0.250, 0.878, 0.815, 1.0  ), //TURQUOISE
    Color::new(0.5  , 0.5  , 0.0  , 1.0  ), //OLIVE
    Color::new(0.5  , 0.0  , 0.0  , 1.0  ), //MAROON
    Color::new(0.0  , 1.0  , 1.0  , 1.0  ), //AQUA
    Color::new(0.0  , 0.5  , 0.5  , 1.0  ), //TEAL
    Color::new(1.0  , 0.843, 0.0  , 1.0  ), //GOLD
    Color::new(0.75 , 0.75 , 0.75 , 1.0  ), //SILVER
    Color::new(1.0  , 0.498, 0.313, 1.0  ), //CORAL
    Color::new(0.933, 0.509, 0.933, 1.0  ), //VIOLET
    Color::new(0.596, 1.0  , 0.596, 1.0  ), //MINT
    Color::new(0.960, 0.960, 0.862, 1.0  ), //BEIGE
    Color::new(0.980, 0.501, 0.447, 1.0  ), //SALMON
    Color::new(0.956, 0.643, 0.376, 1.0  ), //SANDYBROWN
    Color::new(0.294, 0.0  , 0.509, 1.0  ), //INDIGO
    Color::new(0.862, 0.078, 0.235, 1.0  ), //CRIMSON
    Color::new(0.941, 0.901, 0.549, 1.0  ), //KHAKI
    Color::new(0.866, 0.627, 0.866, 1.0  ), //PLUM
    Color::new(0.823, 0.411, 0.117, 1.0  ), //CHOCOLATE
    Color::new(0.0  , 0.392, 0.0  , 1.0  ), //DARKGREEN
    Color::new(1.0  , 0.549, 0.0  , 1.0  ), //DARKORANGE
];

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum FluidPacket {
    Empty,
    Fluid { color_id: usize },
}
impl FluidPacket {
    pub fn new(color_id: usize) -> Self {
        FluidPacket::Fluid { color_id }
    }

    pub fn new_from_repr(repr: &str) -> Self {
        let s = repr.trim();
        if s.is_empty() || s == "." {
            return FluidPacket::Empty;
        }

        // Allow multi-character labels: A..Z, AA, AB, ... (Excel-style).
        // Any non A-Z character makes the repr invalid and results in Empty.
        match Self::letters_to_color_id(s) {
            Some(id) => FluidPacket::Fluid { color_id: id },
            None => FluidPacket::Empty,
        }
    }

    /// Convert a single letter (A-Z) into a 0-based id.
    pub fn letter_to_color_id(ch: char) -> Option<usize> {
        if !ch.is_ascii_alphabetic() {
            return None;
        }
        let up = ch.to_ascii_uppercase();
        Some((up as u8 - b'A') as usize)
    }

    /// Convert a letter sequence like "A", "Z", "AA" into a 0-based id.
    /// Uses Excel-style base-26 numbering: A=0, B=1, ..., Z=25, AA=26, AB=27, ...
    fn letters_to_color_id(s: &str) -> Option<usize> {
        let mut acc: usize = 0;
        let mut saw_any = false;

        for ch in s.chars() {
            let digit = Self::letter_to_color_id(ch)?; // 0..25
            // Convert to 1..26 for Excel-style accumulation.
            acc = acc
                .checked_mul(26)?
                .checked_add(digit + 1)?;
            saw_any = true;
        }

        if !saw_any {
            return None;
        }

        // Back to 0-based.
        acc.checked_sub(1)
    }

    pub fn is_empty(&self) -> bool {
        matches!(self, FluidPacket::Empty)
    }
    pub fn get_color_id(&self) -> Option<usize> {
        match self {
            FluidPacket::Fluid { color_id } => Some(*color_id),
            FluidPacket::Empty => None,
        }
    }
    pub fn get_letter_representation(&self) -> String {
        let letters = b'A'..=b'Z';
        let letter_vec: Vec<u8> = letters.collect();
        let len = letter_vec.len();

        let mut chars = Vec::new();
        let mut id = match self.get_color_id() {
            None => return ".".to_string(),
            Some(id) => id + 1, // 1-based for easier calculation
        };

        while id > 0 {
            let rem = (id - 1) % len;
            chars.push(letter_vec[rem] as char);
            id = (id - 1) / len;
        }

        chars.iter().rev().collect()
    }
    pub fn get_color(&self) -> Option<Color> {
        match self {
            FluidPacket::Fluid { color_id } => Some(
                FLUID_COLORS[color_id % FLUID_COLORS.len()]
            ),
            FluidPacket::Empty => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FluidContainer {
    packets: Vec<FluidPacket>,
    capacity: usize,
}
impl FluidContainer {
    pub fn new(capacity: usize) -> Self {
        Self {
            packets: vec![FluidPacket::Empty; capacity],
            capacity,
        }
    }

    pub fn new_from_repr(repr: &str) -> Self {
        let mut packets = Vec::new();
        let use_commas = repr.contains(',');
        if use_commas {
            for token in repr.split(',') {
                let packet = FluidPacket::new_from_repr(token);
                packets.push(packet);
            }
        } else {
            for ch in repr.chars() {
                let packet = FluidPacket::new_from_repr(&ch.to_string());
                packets.push(packet);
            }
        }
        let capacity = packets.len();
        Self {
            packets,
            capacity,
        }
    }

    pub fn resize(&mut self, new_capacity: usize) {
        if new_capacity > self.capacity {
            self.packets
                .extend(vec![FluidPacket::Empty; new_capacity - self.capacity]);
        } else if new_capacity < self.capacity {
            self.packets.truncate(new_capacity);
        }
        self.capacity = new_capacity;
    }

    pub fn change_capacity(&mut self, delta: isize) {
        let new_capacity = if delta.is_negative() {
            self.capacity.saturating_sub(delta.wrapping_abs() as usize)
        } else {
            self.capacity.saturating_add(delta as usize)
        };
        self.resize(new_capacity);
    }

    pub fn add_fluid(&mut self, packet: FluidPacket) -> bool {
        for p in &mut self.packets {
            if p.is_empty() {
                *p = packet;
                return true;
            }
        }
        false
    }

    pub fn push_fluid(&mut self, packet: FluidPacket) -> bool {
        if self.is_empty() || self.get_top_fluid() == Some(packet) {
            return self.add_fluid(packet);
        }
        false
    }

    pub fn pop_fluid(&mut self) -> Option<FluidPacket> {
        for packet in self.packets.iter_mut().rev() {
            if let FluidPacket::Fluid { color_id } = packet {
                let color_id = *color_id;
                *packet = FluidPacket::Empty;
                return Some(FluidPacket::Fluid { color_id });
            }
        }
        None
    }

    #[allow(dead_code)]
    pub fn is_full(&self) -> bool {
        self.packets.iter().all(|p| !p.is_empty())
    }

    pub fn is_empty(&self) -> bool {
        self.packets.iter().all(|p| p.is_empty())
    }

    pub fn get_empty_space(&self) -> usize {
        self.packets.iter().filter(|p| p.is_empty()).count()
    }

    pub fn get_capacity(&self) -> usize {
        self.capacity
    }

    pub fn get_filled_amount(&self) -> usize {
        self.get_capacity() - self.get_empty_space()
    }

    pub fn get_top_fluid(&self) -> Option<FluidPacket> {
        for packet in self.packets.iter().rev() {
            if let FluidPacket::Fluid { color_id: _ } = packet {
                return Some(*packet);
            }
        }
        None
    }

    pub fn get_top_fluid_depth(&self) -> usize {
        let mut depth = 0;
        let mut packets = self.packets.iter().rev();
        let top_color_id = loop {
            match packets.next() {
                Some(FluidPacket::Empty) => continue,
                Some(FluidPacket::Fluid { color_id }) => break *color_id,
                None => return 0,
            }
        };
        depth += 1;
        for packet in packets {
            match packet {
                FluidPacket::Fluid { color_id } if *color_id == top_color_id => depth += 1,
                _ => break,
            }
        }
        depth
    }

    pub fn get_packets(&self) -> &Vec<FluidPacket> {
        &self.packets
    }

    pub fn get_pourable_amount(&self, other: &FluidContainer) -> usize {
        if self.get_top_fluid() != other.get_top_fluid() && !other.is_empty() {
            return 0;
        }
        let depth = self.get_top_fluid_depth();
        let space = other.get_empty_space();
        depth.min(space)
    }

    pub fn could_pour_into(&self, other: &FluidContainer) -> bool {
        self.get_pourable_amount(other) > 0
    }

    pub fn pour_into(&mut self, other: &mut FluidContainer) -> bool {
        let transfer_amount = self.get_pourable_amount(other);
        if transfer_amount == 0 {
            return false;
        }
        for _ in 0..transfer_amount {
            if let Some(packet) = self.pop_fluid() {
                other.push_fluid(packet);
            }
        }
        true
    }
    
    pub fn could_reverse_pour_into(&self, other: &FluidContainer) -> bool {
        self.get_reverse_pourable_amount(other) > 0
    }

    pub fn get_reverse_pourable_amount(&self, other: &FluidContainer) -> usize {
        let space = other.get_empty_space();
        let mut self_depth = self.get_top_fluid_depth();
        if self_depth != self.get_filled_amount() { // Need to leave at least one packet behind to pour back, or empty
            self_depth -= 1;
        }
        space.min(self_depth)
    }


    pub fn reverse_pour_into(&mut self, other: &mut FluidContainer, amount: usize) -> bool {
        let transfer_amount = self.get_reverse_pourable_amount(other).min(amount);
        if transfer_amount == 0 {
            return false;
        }
        for _ in 0..transfer_amount {
            if let Some(packet) = self.pop_fluid() {
                other.add_fluid(packet);
            }
        }
        true
    }

    pub fn get_text_representation(&self) -> String {
        let mut repr = vec![];
        for packet in &self.packets {
            repr.push(packet.get_letter_representation());
        }
        let has_multi_char = repr.iter().any(|s| s.len() > 1);
        let separator = if has_multi_char { "," } else { "" };
        repr.join(separator)
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ControlAction {
    SelectColor(usize),
    SelectContainer(usize),
    Deselect,
    PourInto(usize,usize),
    ReversePour(usize,usize,usize),
    Undo,
    Redo,
    ToggleEditor,
    CopyState,
    // Editor actions
    PasteState,
    AddColor(usize,usize),
    RemoveColor(usize),
    AddContainer,
    RemoveContainer,
    ExpandContainer,
    ShrinkContainer,
}
#[derive(Clone, Debug, PartialEq)]
pub struct Button {
    label: String,
    action: ControlAction,
    color: Color,
}
impl Button {
    pub fn new(label: &str, action: ControlAction, color: Color) -> Self {
        Self {
            label: label.to_string(),
            action,
            color,
        }
    }
    pub fn get_action(&self) -> ControlAction {
        self.action
    }
    pub fn get_label(&self) -> &str {
        &self.label
    }
    pub fn get_color(&self) -> Color {
        self.color
    }
    pub fn editor_mode(&self) -> bool {
        matches!(self.action, ControlAction::AddColor(_,_) | ControlAction::RemoveColor(_) | ControlAction::AddContainer | ControlAction::RemoveContainer | ControlAction::ExpandContainer | ControlAction::ShrinkContainer | ControlAction::PasteState)
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum HitItem {
    Button { function: ControlAction },
    Container { index: usize },
    Swatch { index: usize },
    #[allow(dead_code)]
    PacketInContainer { container_index: usize, packet_index: usize }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct HitRecord {
    pub rect: Rect,
    pub item: HitItem,
    #[allow(dead_code)]
    pub order: usize,
}

#[derive(Default)]
pub struct HitTestRegistry {
    items: Vec<HitRecord>,
}

impl HitTestRegistry {
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    pub fn clear(&mut self) {
        self.items.clear();
    }

    pub fn push(&mut self, rect: Rect, item: HitItem, order: usize) {
        self.items.push(HitRecord { rect, item, order });
    }

    /// Returns the topmost item under the point (highest draw order).
    pub fn hit_test(&self, x: f32, y: f32) -> Option<&HitRecord> {
        self.items
            .iter()
            .rev() // last drawn wins
            .find(|r| r.rect.contains(vec2(x, y)))
    }

    /// Returns all items under the point, ordered topmost-first.
    #[allow(dead_code)]
    pub fn hit_test_all(&self, x: f32, y: f32) -> Vec<&HitRecord> {
        self.items
            .iter()
            .rev()
            .filter(|r| r.rect.contains(vec2(x, y)))
            .collect()
    }
}
