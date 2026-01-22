use macroquad::prelude::*;
use clipboard_rs::{Clipboard, ClipboardContext};

use std::{
    collections::HashMap,
    sync::{Mutex, OnceLock},
};

/// Cache key for text sizing.
///
/// Width/height are quantized to whole pixels to avoid float noise causing cache misses.
#[derive(Hash, PartialEq, Eq, Clone, Debug)]
struct TextCacheKey {
    text: String,
    w_px: u16,
    h_px: u16,
}

type TextMaxSize = (f32, f32, f32);
type TextMaxSizeCache = HashMap<TextCacheKey, TextMaxSize>;

static TEXT_MAX_SIZE_CACHE: OnceLock<Mutex<TextMaxSizeCache>> = OnceLock::new();

fn text_cache() -> &'static Mutex<TextMaxSizeCache> {
    TEXT_MAX_SIZE_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn clear_text_cache() {
    if let Ok(mut cache) = text_cache().lock() {
        cache.clear();
    }
}

fn text_max_size_cached(text: &str, rect_width: f32, rect_height: f32) -> TextMaxSize {
    let w_px = rect_width.round().clamp(0.0, u16::MAX as f32) as u16;
    let h_px = rect_height.round().clamp(0.0, u16::MAX as f32) as u16;

    let key = TextCacheKey {
        text: text.to_string(),
        w_px,
        h_px,
    };

    if let Ok(cache) = text_cache().lock()
        && let Some(v) = cache.get(&key)
    {
        return *v;
    }

    let v = text_max_size(text, rect_width, rect_height);

    if let Ok(mut cache) = text_cache().lock() {
        cache.insert(key, v);
    }

    v
}

// Keeps track of window/screen resizing so we can clear the cache only when needed.
struct ResizeCacheInvalidation {
    last_w: u16,
    last_h: u16,
    last_resize_time: f64,
    last_clear_time: f64,
    resize_recent_until: f64,
}

impl ResizeCacheInvalidation {
    fn new() -> Self {
        let w = screen_width().round().max(0.0) as u16;
        let h = screen_height().round().max(0.0) as u16;
        let now = get_time();
        Self {
            last_w: w,
            last_h: h,
            last_resize_time: now,
            last_clear_time: now,
            // Consider resize "recent" for 2 seconds after the last size change.
            resize_recent_until: now,
        }
    }

    fn update(&mut self) {
        let w = screen_width().round().max(0.0) as u16;
        let h = screen_height().round().max(0.0) as u16;
        let now = get_time();

        if w != self.last_w || h != self.last_h {
            self.last_w = w;
            self.last_h = h;
            self.last_resize_time = now;
            self.resize_recent_until = now + 2.0;
        }

        // Only clear around once a second, and only while a resize has happened recently.
        if now < self.resize_recent_until && (now - self.last_clear_time) >= 1.0 {
            clear_text_cache();
            self.last_clear_time = now;
        }
    }
}

fn text_max_size(text: &str, rect_width: f32, rect_height: f32) -> (f32, f32, f32) {
    // Measure at a large reference size, then scale down to fit in one pass
    let reference_size = 100u16;
    let dimensions = measure_text(text, None, reference_size, 1.0);

    let scale_x = rect_width / dimensions.width;
    let scale_y = rect_height / dimensions.height;
    let scale = scale_x.min(scale_y);

    let optimal_size = (reference_size as f32 * scale).floor().max(1.0);
    let final_dimensions = measure_text(text, None, optimal_size as u16, 1.0);

    let x = (rect_width - final_dimensions.width) / 2.0;
    let y = (rect_height + final_dimensions.height) / 2.0 - 2.0;
    (optimal_size, x, y)
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
enum FluidPacket {
    Empty,
    Fluid { color_id: usize },
}
impl FluidPacket {
    fn new(color_id: usize) -> Self {
        FluidPacket::Fluid { color_id }
    }
    fn is_empty(&self) -> bool {
        matches!(self, FluidPacket::Empty)
    }
    fn draw(&self, x: f32, y: f32, width: f32, height: f32) {
        match self {
            FluidPacket::Empty => {
                draw_rectangle(x, y, width, height, GRAY);
            }
            FluidPacket::Fluid { color_id } => {
                let color = color_from_id(*color_id);
                draw_rectangle(x, y, width, height, color);

                let letter = letter_from_id(*color_id);
                let (font_size, text_x, text_y) = text_max_size_cached(&letter, width, height);
                draw_text(&letter, x + text_x, y + text_y, font_size, BLACK);
            }
        }
        draw_rectangle_lines(x, y, width, height, 1.0, BLACK);
    }
}

#[derive(Clone, Debug)]
struct FluidContainer {
    packets: Vec<FluidPacket>,
    capacity: usize,
}
impl FluidContainer {
    fn new(capacity: usize) -> Self {
        Self {
            packets: vec![FluidPacket::Empty; capacity],
            capacity,
        }
    }

    fn push_fluid(&mut self, color_id: usize) -> bool {
        for packet in &mut self.packets {
            if packet.is_empty() {
                *packet = FluidPacket::Fluid { color_id };
                return true;
            }
        }
        false
    }

    fn pop_fluid(&mut self) -> Option<usize> {
        for packet in self.packets.iter_mut().rev() {
            if let FluidPacket::Fluid { color_id } = packet {
                let color_id = *color_id;
                *packet = FluidPacket::Empty;
                return Some(color_id);
            }
        }
        None
    }

    fn is_full(&self) -> bool {
        self.packets
            .iter()
            .all(|p| matches!(p, FluidPacket::Fluid { .. }))
    }
    fn is_empty(&self) -> bool {
        self.packets.iter().all(|p| p.is_empty())
    }

    fn top_color_id(&self) -> Option<usize> {
        for packet in self.packets.iter().rev() {
            if let FluidPacket::Fluid { color_id } = packet {
                return Some(*color_id);
            }
        }
        None
    }

    fn top_color_depth(&self) -> usize {
        let mut depth = 0;
        let mut packets = self.packets.iter().rev();

        // Skip trailing empties
        let top_color_id = loop {
            match packets.next() {
                Some(FluidPacket::Empty) => continue,
                Some(FluidPacket::Fluid { color_id }) => break *color_id,
                None => return 0,
            }
        };

        // Count contiguous packets with the same color_id
        depth += 1;
        for packet in packets {
            match packet {
                FluidPacket::Fluid { color_id } if *color_id == top_color_id => depth += 1,
                _ => break,
            }
        }
        depth
    }

    fn empty_space(&self) -> usize {
        self.packets.iter().filter(|p| p.is_empty()).count()
    }

    fn pour_into(&mut self, other: &mut FluidContainer) -> bool {
        if self.is_empty() || other.is_full() {
            return false;
        }

        let self_top_color_id = match self.top_color_id() {
            Some(color_id) => color_id,
            None => return false,
        };

        if let Some(other_color_id) = other.top_color_id()
            && other_color_id != self_top_color_id
        {
            return false;
        }

        let depth = self.top_color_depth();
        let space = other.empty_space();
        let transfer_amount = depth.min(space);

        for _ in 0..transfer_amount {
            if let Some(color_id) = self.pop_fluid() {
                other.push_fluid(color_id);
            }
        }

        true
    }

    fn draw(&self, x: f32, y: f32, width: f32, height: f32) {
        let packet_height = height / self.capacity as f32;
        for (i, packet) in self.packets.iter().enumerate() {
            let packet_y = y + height - (i as f32 + 1.0) * packet_height;
            packet.draw(x, packet_y, width, packet_height);
        }
    }

    fn text_representation(&self) -> String {
        let mut repr = String::new();
        for packet in &self.packets {
            match packet {
                FluidPacket::Empty => repr.push('.'),
                FluidPacket::Fluid { color_id } => {
                    let letter = letter_from_id(*color_id);
                    repr.push_str(&letter);
                }
            }
        }
        repr
    }
}

// `macroquad::prelude` provides only a small set of named colors. Define the
// extra ones we want to use in the lookup table.
const NAVY: Color = Color::new(0.0, 0.0, 0.5, 1.0);
const OLIVE: Color = Color::new(0.5, 0.5, 0.0, 1.0);
const MAROON: Color = Color::new(0.5, 0.0, 0.0, 1.0);
const AQUA: Color = Color::new(0.0, 1.0, 1.0, 1.0);
const CYAN: Color = Color::new(0.0, 1.0, 1.0, 1.0);
const TEAL: Color = Color::new(0.0, 0.5, 0.5, 1.0);
const GOLD: Color = Color::new(1.0, 0.843_137_26, 0.0, 1.0);
const SILVER: Color = Color::new(0.75, 0.75, 0.75, 1.0);
const INDIGO: Color = Color::new(0.294_117_66, 0.0, 0.509_803_95, 1.0);
const VIOLET: Color = Color::new(0.933_333_34, 0.509_803_95, 0.933_333_34, 1.0);
const CORAL: Color = Color::new(1.0, 0.498_039_22, 0.313_725_5, 1.0);
const SALMON: Color = Color::new(0.980_392_16, 0.501_960_8, 0.447_058_83, 1.0);
const TURQUOISE: Color = Color::new(0.250_980_4, 0.878_431_4, 0.815_686_3, 1.0);
const MINT: Color = Color::new(0.596_078_46, 1.0, 0.596_078_46, 1.0);
const BEIGE: Color = Color::new(0.960_784_3, 0.960_784_3, 0.862_745_1, 1.0);
const CHOCOLATE: Color = Color::new(0.823_529_4, 0.411_764_7, 0.117_647_06, 1.0);
const CRIMSON: Color = Color::new(0.862_745_1, 0.078_431_375, 0.235_294_12, 1.0);
const KHAKI: Color = Color::new(0.941_176_5, 0.901_960_8, 0.549_019_63, 1.0);
const PLUM: Color = Color::new(0.866_666_7, 0.627_451, 0.866_666_7, 1.0);
const SANDYBROWN: Color = Color::new(0.956_862_75, 0.643_137_3, 0.376_470_6, 1.0);
const DARKGREEN: Color = Color::new(0.0, 0.392_156_87, 0.0, 1.0);
const DARKORANGE: Color = Color::new(1.0, 0.549_019_63, 0.0, 1.0);

const FLUID_COLORS: [Color; 32] = [
    RED,
    GREEN,
    BLUE,
    YELLOW,
    ORANGE,
    PURPLE,
    CYAN,
    MAGENTA,
    LIME,
    PINK,
    TEAL,
    BROWN,
    NAVY,
    OLIVE,
    MAROON,
    AQUA,
    GOLD,
    SILVER,
    INDIGO,
    VIOLET,
    CORAL,
    SALMON,
    TURQUOISE,
    MINT,
    BEIGE,
    CHOCOLATE,
    CRIMSON,
    KHAKI,
    PLUM,
    SANDYBROWN,
    DARKGREEN,
    DARKORANGE,
];

fn color_from_id(id: usize) -> Color {
    FLUID_COLORS[id % FLUID_COLORS.len()]
}
fn letter_from_id(mut id: usize) -> String {
    let letters = b'A'..=b'Z';
    let letter_vec: Vec<u8> = letters.collect();
    let len = letter_vec.len();

    let mut chars = Vec::new();
    id += 1; // 1-based for easier calculation

    while id > 0 {
        let rem = (id - 1) % len;
        chars.push(letter_vec[rem] as char);
        id = (id - 1) / len;
    }

    chars.iter().rev().collect()
}
#[derive(Clone)]
enum ControlAction {
    SelectColor(usize),
    SelectContainer(usize),
    AddContainer,
    RemoveContainer,
    ExpandContainer,
    ShrinkContainer,
    CopyState,
    PasteState,
}
struct Button {
    label: String,
    action: ControlAction,
    color: Color,
}
struct Controls {
    containers: Vec<FluidContainer>,
    swatch_colors: Vec<FluidPacket>,
    buttons: Vec<Button>,
    selected_color: Option<usize>,
    selected_container: Option<usize>,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    swatch_width: f32,
    controls_height: f32,
}
impl Controls {
    fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        let mut colors = Vec::new();
        colors.push(FluidPacket::Empty);
        for i in 0..8 {
            colors.push(FluidPacket::new(i));
        }
        Controls {
            swatch_colors: colors,
            selected_color: None,
            selected_container: None,
            buttons: vec![
                Button {
                    label: "Add".to_string(),
                    action: ControlAction::AddContainer,
                    color: GREEN,
                },
                Button {
                    label: "Remove".to_string(),
                    action: ControlAction::RemoveContainer,
                    color: RED,
                },
                Button {
                    label: "Expand".to_string(),
                    action: ControlAction::ExpandContainer,
                    color: YELLOW,
                },
                Button {
                    label: "Shrink".to_string(),
                    action: ControlAction::ShrinkContainer,
                    color: ORANGE,
                },
                Button {
                    label: "Copy".to_string(),
                    action: ControlAction::CopyState,
                    color: SKYBLUE,
                },
                Button {
                    label: "Paste".to_string(),
                    action: ControlAction::PasteState,
                    color: PURPLE,
                },
            ],
            x,
            y,
            width,
            height,
            swatch_width: 0.6,
            controls_height: 0.2,
            containers: Vec::new(),
        }
    }
    fn reposition(&mut self, x: f32, y: f32, width: f32, height: f32) {
        self.x = x;
        self.y = y;
        self.width = width;
        self.height = height;
    }
    fn auto_reposition(&mut self) {
        self.reposition(10.0, 10.0, screen_width()-20.0, screen_height()-20.0);
    }
    fn draw_screen(&self) {
        // Draw containers, color swatches, and buttons using self's layout
        self.draw_containers();
        self.draw_swatches();
        self.draw_buttons();
    }

    fn draw_swatches(&self) {
        let swatch_width = self.width * self.swatch_width;
        let swatch_height = self.height * self.controls_height;
        let color_count = self.swatch_colors.len();
        let single_swatch_width = swatch_width / color_count as f32;
        let swatch_y = self.y + self.height - swatch_height;
        if let Some(selected) = self.selected_color
        {
            let swatch_x = self.x + selected as f32 * single_swatch_width;
            draw_rectangle(
                swatch_x,
                swatch_y,
                single_swatch_width,
                swatch_height,
                WHITE,
            );
        }
        for (i, color) in self.swatch_colors.iter().enumerate() {
            let swatch_x = self.x + i as f32 * single_swatch_width;
            if self.selected_color == Some(i)
            {
                color.draw(swatch_x + 4.0, swatch_y + 4.0, single_swatch_width - 8.0, swatch_height - 8.0);
                continue;
            }
            color.draw(swatch_x, swatch_y, single_swatch_width, swatch_height);
        }
    }

    fn draw_buttons(&self) {
        let buttons_width = self.width * (1.0 - self.swatch_width);
        let button_start_x = self.x + self.width * self.swatch_width;
        let button_width = buttons_width / self.buttons.len() as f32;
        let button_height = self.height * self.controls_height;
        let button_y = self.y + self.height - button_height;

        for (i, button) in self.buttons.iter().enumerate() {
            let button_x = button_start_x + i as f32 * button_width;
            draw_rectangle(button_x, button_y, button_width, button_height, button.color);
            let (font_size, text_x, text_y) = text_max_size_cached(&button.label, button_width, button_height);
            draw_text(&button.label, button_x + text_x, button_y + text_y, font_size, DARKGRAY);
        }
    }
    fn draw_containers(&self) {
        let containers = &self.containers;
        // This function will draw all the containers into a box, keeping them evenly spaced, with a max of 8 per row.
        let max_per_row = 8;
        let horizontal_spacing = 10.0;
        let vertical_spacing = 10.0;
        let container_width = (self.width / max_per_row as f32) - horizontal_spacing;
        let containers_height: f32 = self.height - (self.height * self.controls_height) - vertical_spacing;
        let container_height = (containers_height / ((containers.len() as f32 / max_per_row as f32).ceil())) - vertical_spacing;
        if let Some(selected) = self.selected_container && selected < containers.len()
        {
            let container_x = self.x + (selected % max_per_row) as f32 * (container_width + horizontal_spacing);
            let container_y = self.y + (selected / max_per_row) as f32 * (container_height + vertical_spacing);
            draw_rectangle(
            container_x - 4.0,
            container_y - 4.0,
            container_width + 8.0,
            container_height + 8.0,
            WHITE,
            );
        }
        for (i, container) in containers.iter().enumerate() {
            let container_x = self.x + (i % max_per_row) as f32 * (container_width + horizontal_spacing);
            let container_y = self.y + (i / max_per_row) as f32 * (container_height + vertical_spacing);
            container.draw(container_x, container_y, container_width, container_height);
        }
    }
    fn add_container(&mut self, capacity: usize, position: Option<usize>) {
        if let Some(pos) = position && pos <= self.containers.len() {
                self.containers.insert(pos, FluidContainer::new(capacity));
                return;
            }
        self.containers.push(FluidContainer::new(capacity));
    }

    fn detect_click(&self, click_x: f32, click_y: f32) -> Option<ControlAction> {
        if click_y < self.y || click_y > self.y + self.height {
            return None;
        }
        if click_x < self.x || click_x > self.x + self.width {
            return None;
        }
        // Check color swatches
        let swatch_width = self.width * self.swatch_width;
        let swatch_height = self.height * self.controls_height;
        let swatch_y = self.y + self.height - swatch_height;
        if click_y >= swatch_y {
            let color_count = self.swatch_colors.len();
            let single_swatch_width = swatch_width / color_count as f32;
            if click_x >= self.x && click_x <= self.x + swatch_width {
                let index = ((click_x - self.x) / single_swatch_width).floor() as usize;
                if index < color_count {
                    return Some(ControlAction::SelectColor(index));
                }
            }
        }
        // Check buttons
        let buttons_width = self.width * (1.0 - self.swatch_width);
        let button_start_x = self.x + self.width * self.swatch_width;
        let button_width = buttons_width / self.buttons.len() as f32;
        let button_height = self.height * self.controls_height;
        let button_y = self.y + self.height - button_height;
        if click_y >= button_y && click_x >= button_start_x && click_x <= button_start_x + buttons_width {
            let index = ((click_x - button_start_x) / button_width).floor() as usize;
            if index < self.buttons.len() {
                return Some(self.buttons[index].action.clone());
            }
        }
        // Check containers
        let containers = &self.containers;
        let max_per_row = 8;
        let horizontal_spacing = 10.0;
        let vertical_spacing = 10.0;
        let container_width = (self.width / max_per_row as f32) - horizontal_spacing;
        let containers_height: f32 = self.height - (self.height * self.controls_height) - vertical_spacing;
        let container_height = (containers_height / ((containers.len() as f32 / max_per_row as f32).ceil())) - vertical_spacing;
        for (i, _container) in containers.iter().enumerate() {
            let container_x = self.x + (i % max_per_row) as f32 * (container_width + horizontal_spacing);
            let container_y = self.y + (i / max_per_row) as f32 * (container_height + vertical_spacing);
            if click_x >= container_x
            && click_x <= container_x + container_width
            && click_y >= container_y
            && click_y <= container_y + container_height
            {
            return Some(ControlAction::SelectContainer(i));
            }
        }
        None
    }

    fn handle_click(&mut self, click_x: f32, click_y: f32) {
        if let Some(selected) = self.selected_container && selected >= self.containers.len() {
            self.selected_container = None;
        }
        if let Some(selected) = self.selected_color && selected >= self.swatch_colors.len() {
            self.selected_color = None;
        }
        if let Some(action) = self.detect_click(click_x, click_y) {
            match action {
                ControlAction::SelectColor(index) => {
                    // guard clause
                    if index >= self.swatch_colors.len() {
                        return;
                    }
                    // deselect if already selected
                    if self.selected_color == Some(index) {
                        self.selected_color = None;
                        return;
                    }
                    // place fluid in selected container if one is selected
                    if let Some(selected) = self.selected_container {
                        match &self.swatch_colors[index] {
                            FluidPacket::Fluid { color_id } => {
                                self.containers[selected].push_fluid(*color_id);
                            }
                            FluidPacket::Empty => {
                                let _ = self.containers[selected].pop_fluid();
                            }
                        };
                        return;
                    }
                    // select color
                    self.selected_color = Some(index);
                }
                ControlAction::SelectContainer(container_id) => {
                    // guard clause
                    if container_id >= self.containers.len() {
                        return;
                    }
                    // deselect if already selected
                    if self.selected_container == Some(container_id) {
                        self.selected_container = None;
                        return;
                    }
                    // place fluid if color selected
                    if let Some(selected_color) = self.selected_color {
                        match &self.swatch_colors[selected_color] {
                            FluidPacket::Fluid { color_id } => {
                                self.containers[container_id].push_fluid(*color_id);
                            }
                            FluidPacket::Empty => {
                                let _ = self.containers[container_id].pop_fluid();
                            }
                        };
                        return;
                    }
                    // pour from selected container if one is selected
                    if let Some(selected) = self.selected_container {
                        let (left, right) = if selected < container_id {
                            let (l, r) = self.containers.split_at_mut(container_id);
                            (&mut l[selected], &mut r[0])
                        } else {
                            let (l, r) = self.containers.split_at_mut(selected);
                            (&mut r[0], &mut l[container_id])
                        };
                        if left.pour_into(right) {
                            self.selected_container = None;
                            return;
                        }
                    }
                    // select container
                    self.selected_container = Some(container_id);
                }
                ControlAction::AddContainer => {
                    if let Some(selected) = self.selected_container {
                        let capacity = self.containers[selected].capacity;
                        self.add_container(capacity, Some(selected));
                        return;
                    }
                    if !self.containers.is_empty() {
                        let capacity = self.containers[self.containers.len() - 1].capacity;
                        self.add_container(capacity, None);
                        return;
                    }
                    self.add_container(5, None);
                    
                }
                ControlAction::RemoveContainer => {
                    if let Some(selected) = self.selected_container {
                        self.containers.remove(selected);
                        if self.selected_container == Some(selected) && selected >= self.containers.len() {
                            self.selected_container = self.containers.len().checked_sub(1);
                        }
                    }
                }
                ControlAction::ExpandContainer => {
                    if let Some(selected) = self.selected_container {
                        self.containers[selected].capacity += 1;
                        self.containers[selected].packets.push(FluidPacket::Empty);
                    }
                }
                ControlAction::ShrinkContainer => {
                    if let Some(selected) = self.selected_container {
                        if self.containers[selected].capacity > 1 {
                            self.containers[selected].capacity -= 1;
                            self.containers[selected].packets.pop();
                        }
                        else {
                            self.containers.remove(selected);
                        }
                    }
                }
                ControlAction::CopyState => {
                    // Format: one container per line, bottom->top packets.
                    // Uses '.' for empty, and letters for colors.
                    let mut out = String::new();
                    for (i, c) in self.containers.iter().enumerate() {
                        if i > 0 {
                            out.push('\n');
                        }
                        out.push_str(&c.text_representation());
                    }
                    self.set_clipboard(&out);
                }
                ControlAction::PasteState => {
                    let text = self.get_clipboard();
                    let mut new_containers: Vec<FluidContainer> = Vec::new();

                    for line in text.lines() {
                        let line = line.trim();
                        if line.is_empty() {
                            continue;
                        }

                        let capacity = line.chars().count();
                        if capacity == 0 {
                            continue;
                        }

                        let mut container = FluidContainer::new(capacity);
                        for ch in line.chars() {
                            if ch == '.' {
                                continue;
                            }
                            // Map letters back to ids (A=0..Z=25). Anything else is ignored.
                            if ch.is_ascii_uppercase() {
                                let id = (ch as u8 - b'A') as usize;
                                let _ = container.push_fluid(id);
                            }
                        }
                        new_containers.push(container);
                    }

                    if !new_containers.is_empty() {
                        self.containers = new_containers;
                        self.selected_container = None;
                        self.selected_color = None;
                    }
                }
            }
        }
    }
    fn get_clipboard(&self) -> String {
        let ctx = ClipboardContext::new().unwrap();
        ctx.get_text().unwrap_or("".to_string())
    }
    fn set_clipboard(&self, content: &str) {
        let ctx = ClipboardContext::new().unwrap();
        let _ = ctx.set_text(content.to_string());
    }
}

#[macroquad::main("Fluid Container Simulation")]
async fn main() {
    let mut controls = Controls::new(
        10.0,
        screen_height() * (2.0 / 3.0) + 10.0,
        screen_width() - 20.0,
        (screen_height() - 20.0) * (1.0 / 3.0) - 10.0,
    );
    controls.add_container(5, None);

    let mut resize_cache_invalidation = ResizeCacheInvalidation::new();

    loop {
        resize_cache_invalidation.update();
        clear_background(BLACK);
        if is_mouse_button_pressed(MouseButton::Left) {
            let (mouse_x, mouse_y) = mouse_position();
            controls.handle_click(mouse_x, mouse_y);
        }
        controls.auto_reposition();
        controls.draw_screen();
        next_frame().await;
    }
}