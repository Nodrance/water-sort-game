use std::vec;

use crate::model::*;
use crate::renderer::Renderer;
use clipboard_rs::{Clipboard, ClipboardContext};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Selection {
    None,
    Container(usize),
    Color(usize),
    #[allow(dead_code)]
    Button(usize),
}

pub struct GameEngine {
    state: GameState,
    starting_state: GameState,
    swatch_colors: Vec<FluidPacket>,
    buttons: Vec<Button>,
    renderer: Renderer,
    selected: Selection,
    undo_enable: bool,
    undo_stack: Vec<GameState>,
    redo_stack: Vec<GameState>,
    editor_mode: bool,
}

impl GameEngine {
    pub fn new(undo_enable: bool) -> Self {
        let gamestate = GameState {
            fluid_containers: vec![FluidContainer::new(5), FluidContainer::new(5)],
        };
        let mut swatch_colors: Vec<FluidPacket> = vec![FluidPacket::Empty];
        for i in 0..10 {
            swatch_colors.push(FluidPacket::new(i));
        }
        let mut buttons = vec![
            Button::new("Add", ControlAction::AddContainer, FLUID_COLORS[3]), // GREEN
            Button::new("Remove", ControlAction::RemoveContainer, FLUID_COLORS[0]), // RED
            Button::new("Expand", ControlAction::ExpandContainer, FLUID_COLORS[1]), // BLUE
            Button::new("Shrink", ControlAction::ShrinkContainer, FLUID_COLORS[2]), // YELLOW

            Button::new("Paste", ControlAction::PasteState, FLUID_COLORS[4]), // PURPLE
            Button::new("Copy", ControlAction::CopyState, FLUID_COLORS[5]), // ORANGE
            Button::new("Editor", ControlAction::ToggleEditor, FLUID_COLORS[6]), // CYAN
        ];
        if undo_enable {
            buttons.push(Button::new("Undo", ControlAction::Undo, FLUID_COLORS[7])); // MAGENTA
            buttons.push(Button::new("Redo", ControlAction::Redo, FLUID_COLORS[8])); // LIME
        }
        buttons.push(Button::new("Reset", ControlAction::Reset, FLUID_COLORS[9])); // PINK

        Self {
            state: gamestate.clone(),
            starting_state: gamestate.clone(),
            swatch_colors,
            buttons,
            renderer: Renderer::new(),
            selected: Selection::None,
            undo_enable,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            editor_mode: true,
        }
    }

    pub fn is_editor_mode(&self) -> bool {
        self.editor_mode
    }

    pub fn render(&mut self) {
        self.renderer.autoset_viewport();
        let (selected_container, selected_swatch, selected_button) = match &self.selected {
            Selection::Container(index) => (Some(*index), None, None),
            Selection::Color(index) => (None, Some(*index), None),
            Selection::Button(index) => (None, None, Some(*index)),
            Selection::None => (None, None, None),
        };
        let containers = &self.state.fluid_containers.iter().collect::<Vec<_>>();
        let buttons = &self.buttons.iter().filter(|b| !b.editor_mode() || self.editor_mode).collect::<Vec<_>>();
        let swatches = if self.editor_mode {
            self.swatch_colors.as_slice()
        } else {
            &[]
        };
        self.renderer.render_game(
            containers,
            swatches,
            buttons,
            selected_container,
            selected_swatch,
            selected_button,
        );
    }

    pub fn handle_click(&mut self, x: f32, y: f32, is_right_click: bool) {
        if let Some(hit) = self.renderer.get_hit_test_registry().hit_test(x, y) {
            self.handle_hit_item(hit.item, is_right_click);
        }
    }

    fn handle_hit_item(&mut self, item: HitItem, is_right_click: bool) {
        let action = match &item {
            HitItem::Button { function } => {
                *function
            }
            HitItem::Container { index } => {
                match &self.selected {
                    Selection::Color(color_index) => {
                        match self.swatch_colors[*color_index] {
                            FluidPacket::Empty => ControlAction::RemoveColor(*index),
                            FluidPacket::Fluid { color_id } => ControlAction::AddColor(*index, color_id)
                        }
                    }
                    Selection::Container(from_index) => {
                        if from_index == index {
                            ControlAction::Deselect
                        } else if is_right_click {
                            ControlAction::ReversePour(*from_index, *index, 1)
                        } else {
                            ControlAction::PourInto(*from_index, *index)
                        }
                    }
                    Selection::Button(_) | Selection::None => {
                        ControlAction::SelectContainer(*index)
                    }
                }
            }
            HitItem::PacketInContainer { container_index: index, packet_index: _ } => {
                self.handle_hit_item(HitItem::Container { index: *index }, is_right_click);
                return;
            }
            HitItem::Swatch { index } => {
                match &self.selected {
                    Selection::Color(selected_index) => {
                        if selected_index == index {
                            ControlAction::Deselect
                        } else {
                            ControlAction::SelectColor(*index)
                        }
                    }
                    Selection::Container(selected_index) => {
                        match self.swatch_colors[*index] {
                            FluidPacket::Empty => ControlAction::RemoveColor(*selected_index),
                            FluidPacket::Fluid { color_id } => ControlAction::AddColor(*selected_index, color_id)
                        }
                    }
                    Selection::Button(_) | Selection::None => {
                        ControlAction::SelectColor(*index)
                    }
                }
            }
        };
        self.handle_game_action(action);
    }

    pub fn handle_game_action(&mut self, action: ControlAction) {
        if matches!(action, 
            ControlAction::PasteState|
            ControlAction::AddColor(_,_)|
            ControlAction::RemoveColor(_)|
            ControlAction::AddContainer|
            ControlAction::RemoveContainer|
            ControlAction::ExpandContainer|
            ControlAction::ShrinkContainer|
            ControlAction::ReversePour(_, _, _)
        ) && !self.is_editor_mode() {
            return;
        }
        match action {
            ControlAction::SelectColor(index) => {
                self.selected = Selection::Color(index);
            }
            ControlAction::SelectContainer(index) => {
                self.selected = Selection::Container(index);
            }
            ControlAction::Deselect => {
                self.selected = Selection::None;
            }
            ControlAction::PourInto(from, to) => {
                if !self.state.fluid_containers[from].could_pour_into(&self.state.fluid_containers[to]) {
                    self.handle_game_action(ControlAction::SelectContainer(to));
                    return;
                }
                self.push_undo_state();
                self.state.apply_move(&MoveAction {
                    from_container: from,
                    to_container: to,
                    amount: 0,
                });
            }
            ControlAction::Undo => {
                self.undo();
            }
            ControlAction::Redo => {
                self.redo();
            }
            ControlAction::Reset => {
                self.push_undo_state();
                self.load_state(self.starting_state.clone());
            }
            ControlAction::ToggleEditor => {
                self.editor_mode = !self.is_editor_mode();
            }
            ControlAction::CopyState => {
                let repr = self.state.get_text_representation();
                self.set_clipboard(&repr);
            }
            // Everything past this point requires editor mode 
            ControlAction::PasteState => {
                if !self.undo_stack.is_empty() {
                    self.push_undo_state();
                }
                let repr = self.get_clipboard();
                let new_state = GameState::new_from_repr(&repr);
                self.load_state(new_state);
                self.starting_state = self.state.clone();
            }
            ControlAction::AddColor(container_id, color_id) => {
                self.push_undo_state();
                let packet = FluidPacket::new(color_id);
                self.state.fluid_containers[container_id].add_fluid(packet);
            }
            ControlAction::RemoveColor(container_id) => {
                self.push_undo_state();
                self.state.fluid_containers[container_id].pop_fluid();
            }
            ControlAction::AddContainer => {
                self.push_undo_state();
                self.add_container();
            }
            ControlAction::RemoveContainer => {
                self.push_undo_state();
                self.remove_container();
            }
            ControlAction::ExpandContainer => {
                self.push_undo_state();
                if let Selection::Container(index) = self.selected {
                    self.state.fluid_containers[index].change_capacity(1);
                }
            }
            ControlAction::ShrinkContainer => {
                self.push_undo_state();
                if let Selection::Container(index) = self.selected {
                    self.state.fluid_containers[index].change_capacity(-1);
                }
            }
            ControlAction::ReversePour(from, to, amount) => {
                if !self.state.fluid_containers[from].could_reverse_pour_into(&self.state.fluid_containers[to]) {
                    self.handle_game_action(ControlAction::SelectContainer(to));
                    return;
                }
                self.push_undo_state();
                self.state.apply_reverse_move(&MoveAction {
                    from_container: from,
                    to_container: to,
                    amount,
                });
            }
        }
        self.render();
    }

    pub fn get_state(&self) -> GameState {
        self.state.clone()
    }

    pub fn load_state(&mut self, state: GameState) {
        self.state = state;
        self.selected = Selection::None;
    }

    fn push_undo_state(&mut self) {
        if self.undo_enable {
            let snapshot = self.get_state();
            self.undo_stack.push(snapshot);
            self.redo_stack.clear();
        }
    }

    fn undo (&mut self) {
        if self.undo_enable && let Some(previous_state) = self.undo_stack.pop() {
            self.redo_stack.push(self.get_state());
            self.state = previous_state;
            self.selected = Selection::None;
        }
    }

    fn redo(&mut self) {
        if self.undo_enable && let Some(next_state) = self.redo_stack.pop() {
            self.undo_stack.push(self.get_state());
            self.state = next_state;
            self.selected = Selection::None;
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

    fn add_container(&mut self) {
        match self.selected {
            Selection::Container(index) => {
                let capacity = self.state.fluid_containers[index].get_capacity();
                self.state.fluid_containers.insert(index + 1, FluidContainer::new(capacity));
                self.selected = Selection::Container(index + 1);
            }
            _ => {
                let capacity = self.state.fluid_containers.last().map_or(5, |c| c.get_capacity());
                self.state.fluid_containers.push(FluidContainer::new(capacity));
                self.selected = Selection::Container(self.state.fluid_containers.len() - 1);
            }
        }
    }
    fn remove_container(&mut self) {
        if let Selection::Container(index) = self.selected {
            if index < self.state.fluid_containers.len() {
                self.state.fluid_containers.remove(index);
                if index >= 1 {
                    self.selected = Selection::Container(index - 1);
                } else {
                    self.selected = Selection::None;
                }
            }
        }
        else {
            self.state.fluid_containers.pop();
        }
    }
}
