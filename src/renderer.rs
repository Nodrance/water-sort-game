use std::{
    collections::HashMap,
    sync::Mutex,
};

use macroquad::prelude::*;
use crate::model::{FluidContainer, FluidPacket, Button, HitItem, HitTestRegistry};

#[derive(Hash, PartialEq, Eq, Clone, Debug)]
struct TextCacheKey {
    text: String,
    w_px: u16,
    h_px: u16,
}
type TextMaxSize = (f32, f32, f32);
pub struct CachedTextSizer {
    final_size_cache: Mutex<HashMap<TextCacheKey, TextMaxSize>>,
    unscaled_size_cache: Mutex<HashMap<String, (f32, f32)>>,
}

impl CachedTextSizer {
    pub fn new() -> Self {
        Self {
            final_size_cache: Mutex::new(HashMap::new()),
            unscaled_size_cache: Mutex::new(HashMap::new()),
        }
    }

    pub fn get_text_max_size(&self, text: &str, rect_width: f32, rect_height: f32) -> TextMaxSize {
        let w_px = rect_width.round().clamp(0.0, u16::MAX as f32) as u16;
        let h_px = rect_height.round().clamp(0.0, u16::MAX as f32) as u16;

        let key = TextCacheKey {
            text: text.to_string(),
            w_px,
            h_px,
        };

        if let Ok(cache) = self.final_size_cache.lock()
            && let Some(cached_size) = cache.get(&key) 
        {
            return *cached_size;
        }

        let text_size = self.measure(text, rect_width, rect_height);
        if let Ok(mut cache) = self.final_size_cache.lock() {
            cache.insert(key, text_size);
        }
        text_size
    }

    fn measure(&self, text: &str, rect_width: f32, rect_height: f32) -> TextMaxSize {
        let reference_size = 100u16;

        let (size_x, size_y) = if let Ok(cache) = self.unscaled_size_cache.lock()
            && let Some(dimensions) = cache.get(text)
        {
            (dimensions.0, dimensions.1)
        }
        else {
            let dimensions = measure_text(text, None, reference_size, 1.0);
            let size_x = dimensions.width;
            let size_y = dimensions.height;

            if let Ok(mut cache) = self.unscaled_size_cache.lock() {
                cache.insert(text.to_string(), (size_x, size_y));
            }
            (size_x, size_y)
        };
        let scale_x = rect_width / size_x;
        let scale_y = rect_height / size_y;
        let optimal_size = reference_size as f32 * scale_x.min(scale_y);
        let final_width = size_x * (optimal_size / reference_size as f32);
        let final_height = size_y * (optimal_size / reference_size as f32);
        let offset_x = (rect_width - final_width) / 2.0;
        let offset_y = (rect_height + final_height) / 2.0;
        (optimal_size, offset_x, offset_y)
    }
}
pub struct Renderer {
    cached_text_sizer: CachedTextSizer,
    hit_test: HitTestRegistry,
    draw_order: usize,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
}
const SELECTION_BORDER: f32 = 4.0;
const TEXT_PADDING: f32 = 10.0;
const CONTAINER_PADDING_HORIZONTAL: f32 = 10.0;
const CONTAINER_PADDING_VERTICAL: f32 = 10.0;
const CONTAINER_LINE_PADDING: f32 = 0.1;
const BUTTON_PADDING_HORIZONTAL: f32 = 10.0;
const BUTTON_HEIGHT: f32 = 0.1;
const SWATCH_HEIGHT: f32 = 0.1;
const GAMEFIELD_PADDING: f32 = 10.0;
const OUTER_MARGIN: f32 = 10.0;

impl Renderer {
    pub fn new() -> Self {
        Self {
            cached_text_sizer: CachedTextSizer::new(),
            hit_test: HitTestRegistry::new(),
            draw_order: 0,
            x: 0.0,
            y: 0.0,
            width: 800.0,
            height: 600.0,
        }
    }

    fn next_order(&mut self) -> usize {
        let o = self.draw_order;
        self.draw_order += 1;
        o
    }

    pub fn get_hit_test_registry(&self) -> &HitTestRegistry {
        &self.hit_test
    }

    pub fn set_viewport(&mut self, x: f32, y: f32, width: f32, height: f32) -> bool {
        if self.x == x && self.y == y && self.width == width && self.height == height {
            return false;
        }
        self.x = x + OUTER_MARGIN;
        self.y = y + OUTER_MARGIN;
        self.width = width - OUTER_MARGIN * 2.0;
        self.height = height - OUTER_MARGIN * 2.0;
        true
    }

    pub fn autoset_viewport(&mut self) -> bool {
        let (screen_w, screen_h) = (screen_width(), screen_height());
        self.set_viewport(0.0, 0.0, screen_w, screen_h)
    }

    pub fn render_game(
        &mut self,
        containers: &[&FluidContainer],
        swatches: &[FluidPacket],
        buttons: &[&Button],        
        selected_container: Option<usize>,
        selected_swatch: Option<usize>,
        selected_button: Option<usize>,
    ) {
        // New frame: reset hit-test registry and draw order.
        self.hit_test.clear();
        self.draw_order = 0;

        clear_background(BLACK);
        let button_area_height = self.height * BUTTON_HEIGHT;
        let swatch_area_height = if !swatches.is_empty() {
            self.height * SWATCH_HEIGHT
        } else {
            0.0
        };
        let container_area_height = self.height - button_area_height - swatch_area_height - 2.0 * GAMEFIELD_PADDING;
        self.render_button_lineup(
            buttons,
            selected_button,
            Rect::new(self.x, self.y, self.width, button_area_height),
        );
        self.render_container_grid(
            containers,
            selected_container,
            6,
            Rect::new(
                self.x,
                self.y + button_area_height + GAMEFIELD_PADDING,
                self.width,
                container_area_height,
            ),
        );
        self.render_color_swatches(
            swatches,
            selected_swatch,
            Rect::new(
                self.x,
                self.y + button_area_height + container_area_height + 2.0 * GAMEFIELD_PADDING,
                self.width,
                swatch_area_height,
            ),
        );
    }

    pub fn render_text(
        &self,
        text: &str,
        rect: Rect,
        color: Color,
    ) {
        if rect.w <= 5.0 || rect.h <= 5.0 {
            return;
        }
        let (optimal_size, x, y) = self
            .cached_text_sizer
            .get_text_max_size(text, rect.w, rect.h);
        draw_text(text, rect.x + x, rect.y + y, optimal_size, color);
    }
    pub fn render_packet(
        &mut self,
        packet: &FluidPacket,
        selected: bool,
        rect: Rect,
        hit_item: Option<HitItem>,
    ) {
        if let Some(item) = hit_item {
            let order = self.next_order();
            self.hit_test.push(rect, item, order);
        }

        match packet {
            FluidPacket::Empty => {
                draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 2.0, DARKGRAY);
            }
            FluidPacket::Fluid { color_id: _ } => {
                let color = packet.get_color().unwrap_or(WHITE);
                draw_rectangle(rect.x, rect.y, rect.w, rect.h, color);
                let text_rect = Rect::new(
                    rect.x + TEXT_PADDING,
                    rect.y + TEXT_PADDING,
                    rect.w - 2.0 * TEXT_PADDING,
                    rect.h - 2.0 * TEXT_PADDING,
                );
                self.render_text(
                    &packet.get_letter_representation(),
                    text_rect,
                    WHITE,
                );
            }
        }
        if selected {
            draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, SELECTION_BORDER, WHITE);
        }
    }
    pub fn render_container(
        &mut self,
        container: &FluidContainer,
        container_index: usize,
        selected: bool,
        rect: Rect,
    ) {
        let order = self.next_order();
        self.hit_test.push(
            rect,
            HitItem::Container {
                index: container_index,
            },
            order,
        );

        let packet_height = rect.h / container.get_capacity() as f32;
        for (i, packet) in container.get_packets().iter().enumerate() {
            let packet_y = rect.y + rect.h - (i as f32 + 1.0) * packet_height;
            if !packet.is_empty() {
                self.render_packet(
                    packet,
                    false,
                    Rect::new(rect.x, packet_y, rect.w, packet_height),
                    Some(HitItem::PacketInContainer {
                        container_index,
                        packet_index: i,
                    }),
                );
            }
            if i < container.get_capacity() - 1 {
                if packet_height <= 2.0 && i % (container.get_capacity() / 10 + 1) != 0 {
                    continue;
                }
                let left_edge = rect.x + (rect.w * CONTAINER_LINE_PADDING);
                let right_edge = rect.x + rect.w - (rect.w * CONTAINER_LINE_PADDING);
                draw_line(left_edge, packet_y, right_edge, packet_y, 2.0, DARKGRAY);
            }
        }
        draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 3.0, DARKGRAY);
        // DEBUG draw entropy near the top
        let entropy = container.get_entropy();
        draw_text(&format!("Entropy: {}", entropy), rect.x + 5.0, rect.y + 5.0, 16.0, WHITE);
        if selected {
            draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, SELECTION_BORDER, WHITE);
        }
    }
    pub fn render_container_lineup(
        &mut self,
        containers: &[&FluidContainer],
        selected: Option<usize>,
        start_index: usize,
        rect: Rect,
    ) {
        let container_count = containers.len() as f32;
        let total_spacing = CONTAINER_PADDING_HORIZONTAL * (container_count - 1.0);
        let container_width = (rect.w - total_spacing) / container_count;
        for (i, container) in containers.iter().enumerate() {
            let container_index = start_index + i;
            let container_x = rect.x + i as f32 * (container_width + CONTAINER_PADDING_HORIZONTAL);
            self.render_container(
                container,
                container_index,
                Some(container_index) == selected,
                Rect::new(container_x, rect.y, container_width, rect.h),
            );
        }
    }
    pub fn render_container_grid(
        &mut self,
        containers: &[&FluidContainer],
        selected: Option<usize>,
        max_columns: usize,
        rect: Rect,
    ) {
        let container_count = containers.len();
        if container_count == 0 {
            return;
        }
        let rows = container_count.div_ceil(max_columns);
        let total_spacing_y = CONTAINER_PADDING_VERTICAL * (rows as f32 - 1.0);
        let container_height = (rect.h - total_spacing_y) / rows as f32;
        let columns = container_count.div_ceil(rows);

        for row in 0..rows {
            let start_idx = row * columns;
            let end_idx = (start_idx + columns).min(container_count);
            let row_containers: Vec<_> = containers[start_idx..end_idx].to_vec();
            let container_y = rect.y + row as f32 * (container_height + CONTAINER_PADDING_VERTICAL);
            self.render_container_lineup(
                &row_containers,
                selected,
                start_idx,
                Rect::new(rect.x, container_y, rect.w, container_height),
            );
        }
    }
    pub fn render_color_swatches (
        &mut self,
        swatches: &[FluidPacket],
        selected: Option<usize>,
        rect: Rect,
    ) {
        let swatch_count = swatches.len() as f32;
        if swatch_count == 0.0 {
            return;
        }
        let spacing = 5.0;
        let total_spacing = spacing * (swatch_count - 1.0);
        let swatch_width = (rect.w - total_spacing) / swatch_count;
        for (i, packet) in swatches.iter().enumerate() {
            let swatch_x = rect.x + i as f32 * (swatch_width + spacing);
            self.render_packet(
                packet,
                Some(i) == selected,
                Rect::new(swatch_x, rect.y, swatch_width, rect.h),
                Some(HitItem::Swatch { index: i }),
            );
        }
    }
    pub fn render_button (
        &mut self,
        button: &Button,
        selected: bool,
        rect: Rect,
    ) {
        let order = self.next_order();
        self.hit_test.push(rect, HitItem::Button { function: button.get_action() }, order);

        draw_rectangle(rect.x, rect.y, rect.w, rect.h, button.get_color());
        draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, 2.0, BLACK);
        let text_rect = Rect::new(
            rect.x + TEXT_PADDING,
            rect.y + TEXT_PADDING,
            rect.w - 2.0 * TEXT_PADDING,
            rect.h - 2.0 * TEXT_PADDING,
        );
        self.render_text(
            button.get_label(),
            text_rect,
            WHITE,
        );
        if selected {
            draw_rectangle_lines(rect.x, rect.y, rect.w, rect.h, SELECTION_BORDER, WHITE);
        }
    }
    pub fn render_button_lineup(
        &mut self,
        buttons: &[&Button],
        selected: Option<usize>,
        rect: Rect,
    ) {
        let button_count = buttons.len() as f32;
        if button_count == 0.0 {
            return;
        }
        let total_spacing = BUTTON_PADDING_HORIZONTAL * (button_count - 1.0);
        let button_width = (rect.w - total_spacing) / button_count;
        for (i, button) in buttons.iter().enumerate() {
            let button_x = rect.x + i as f32 * (button_width + BUTTON_PADDING_HORIZONTAL);
            self.render_button(
                button,
                Some(i) == selected,
                Rect::new(button_x, rect.y, button_width, rect.h),
            );
        }
    }
}
