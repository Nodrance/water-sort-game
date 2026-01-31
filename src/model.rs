use macroquad::{prelude::*};

// FluidPacket

pub const FLUID_COLORS: [Color; 32] = [
    Color::new(1.0  , 0.0  , 0.0  , 1.0  ), //RED
    Color::new(0.0  , 0.0  , 1.0  , 1.0  ), //BLUE
    Color::new(0.9  , 0.9  , 0.0  , 1.0  ), //YELLOW
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

#[derive(Copy, Clone, PartialEq, Eq, Debug, PartialOrd, Ord)]
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
            acc = acc.checked_mul(26)?.checked_add(digit + 1)?;
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
            FluidPacket::Fluid { color_id } => Some(FLUID_COLORS[color_id % FLUID_COLORS.len()]),
            FluidPacket::Empty => None,
        }
    }
}

// FluidContainer

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FluidContainer {
    packets: Vec<FluidPacket>,
    capacity: usize,
}

#[allow(dead_code)]
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
        Self { packets, capacity }
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
            self.capacity
                .saturating_sub(delta.wrapping_abs() as usize)
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
        if self.is_empty() || self.get_top_fluid() == packet {
            return self.add_fluid(packet);
        }
        false
    }

    pub fn pop_fluid(&mut self) -> FluidPacket {
        for packet in self.packets.iter_mut().rev() {
            if let FluidPacket::Fluid { color_id } = packet {
                let color_id = *color_id;
                *packet = FluidPacket::Empty;
                return FluidPacket::Fluid { color_id };
            }
        }
        FluidPacket::Empty
    }

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

    pub fn get_top_fluid(&self) -> FluidPacket {
        for packet in self.packets.iter().rev() {
            if let FluidPacket::Fluid { .. } = packet {
                return *packet;
            }
        }
        FluidPacket::Empty
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
            let packet = self.pop_fluid();
            other.push_fluid(packet);
        }
        true
    }

    pub fn could_reverse_pour_into(&self, other: &FluidContainer) -> bool {
        self.get_reverse_pourable_amount(other) > 0
    }

    pub fn get_reverse_pourable_amount(&self, other: &FluidContainer) -> usize {
        let space = other.get_empty_space();
        let mut self_depth = self.get_top_fluid_depth();
        // Need to leave at least one packet behind to pour back, or empty.
        if self_depth != self.get_filled_amount() {
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
            let packet = self.pop_fluid();
            other.add_fluid(packet);
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

impl PartialOrd for FluidContainer {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for FluidContainer {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.packets.cmp(&other.packets)
    }
}

// Game state / moves

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MoveAction {
    pub from_container: usize,
    pub to_container: usize,
    pub amount: usize,
}

#[derive(Debug, Clone)]
pub struct GameState {
    pub fluid_containers: Vec<FluidContainer>,
}

#[allow(dead_code)]
impl GameState {
    pub fn new_from_repr(repr: &str) -> Self {
        let mut fluid_containers: Vec<FluidContainer> = Vec::new();

        for line in repr.lines() {
            let container = FluidContainer::new_from_repr(line);
            fluid_containers.push(container);
        }
        Self { fluid_containers }
    }

    pub fn get_text_representation(&self) -> String {
        let mut out = String::new();
        for (i, c) in self.fluid_containers.iter().enumerate() {
            if i > 0 {
                out.push('\n');
            }
            out.push_str(&c.get_text_representation());
        }
        out
    }

    pub fn get_available_colors(&self) -> Vec<usize> {
        let mut colors = vec![];
        for container in &self.fluid_containers {
            for packet in container.get_packets() {
                if let FluidPacket::Fluid { color_id } = packet && !colors.contains(color_id) {
                    colors.push(*color_id);
                }
            }
        }
        colors
    }

    pub fn get_available_colors_with_count(&self) -> Vec<(usize, usize)> {
        let mut color_counts = vec![];
        for container in &self.fluid_containers {
            for packet in container.get_packets() {
                if let FluidPacket::Fluid { color_id } = packet {
                    if let Some((_, count)) = color_counts.iter_mut().find(|(id, _)| *id == *color_id) {
                        *count += 1;
                    } else {
                        color_counts.push((*color_id, 1));
                    }
                }
            }
        }
        color_counts
    }

    pub fn get_top_colors(&self) -> Vec<usize> {
        let mut colors = vec![];
        for container in &self.fluid_containers {
            let packet = container.get_top_fluid();
            if let FluidPacket::Fluid { color_id } = packet {
                colors.push(color_id);
            }
        }
        colors
    }

    pub fn get_container_sizes (&self) -> Vec<usize> {
        let mut sizes = vec![];
        for container in &self.fluid_containers {
            sizes.push(container.get_capacity());
        }
        sizes.sort();
        sizes
    }

    pub fn get_possible_moves(&self) -> Vec<MoveAction> {
        let mut moves = vec![];
        for color in self.get_top_colors() {
            for (from_index, from_container) in self.fluid_containers.iter().enumerate() {
                if from_container.is_empty() || from_container.get_top_fluid() != FluidPacket::new(color) {
                    continue;
                }
                for (to_index, to_container) in self.fluid_containers.iter().enumerate() {
                    if from_index == to_index {
                        continue;
                    }
                    let amount = from_container.get_pourable_amount(to_container);
                    if amount > 0 {
                        moves.push(MoveAction {
                            from_container: from_index,
                            to_container: to_index,
                            amount,
                        });
                    }
                }
            }
        }
        moves
    }

    pub fn get_possible_reverse_moves(&self) -> Vec<MoveAction> {
        let mut moves = vec![];
        for color in self.get_top_colors() {
            for (from_index, from_container) in self.fluid_containers.iter().enumerate() {
                if from_container.is_empty() || from_container.get_top_fluid() != FluidPacket::new(color) {
                    continue;
                }
                for (to_index, to_container) in self.fluid_containers.iter().enumerate() {
                    if from_index == to_index {
                        continue;
                    }
                    let amount = from_container.get_reverse_pourable_amount(to_container);
                    if amount > 0 {
                        moves.push(MoveAction {
                            from_container: from_index,
                            to_container: to_index,
                            amount,
                        });
                    }
                }
            }
        }
        moves
    }

    pub fn apply_move(&mut self, action: &MoveAction) {
        let from = action.from_container;
        let to = action.to_container;
        if from < to {
            let (left, right) = self.fluid_containers.split_at_mut(to);
            left[from].pour_into(&mut right[0])
        } else {
            let (left, right) = self.fluid_containers.split_at_mut(from);
            right[0].pour_into(&mut left[to])
        };
    }

    pub fn apply_reverse_move(&mut self, action: &MoveAction) {
        let from = action.from_container;
        let to = action.to_container;
        let amount = action.amount;
        if from < to {
            let (left, right) = self.fluid_containers.split_at_mut(to);
            left[from].reverse_pour_into(&mut right[0], amount);
        } else {
            let (left, right) = self.fluid_containers.split_at_mut(from);
            right[0].reverse_pour_into(&mut left[to], amount);
        };
    }

    pub fn get_sorted_containers(&self) -> Vec<FluidContainer> {
        let mut containers = self.fluid_containers.clone();
        containers.sort();
        containers
    }

    fn fast_is_definitely_solvable(&self) -> bool {
        // Checks if every liquid can perfectly fit into containers of the same size.
        // If this returns true, the puzzle is definitely solvable. If false, may still be solvable.
        // Guaranteed correct if no liquid ends up split across multiple containers.
        let mut container_size_map: std::collections::HashMap<usize, usize> = std::collections::HashMap::new();
        for container in &self.fluid_containers {
            *container_size_map.entry(container.get_capacity()).or_insert(0) += 1;
        }
        let mut liquid_size_map: std::collections::HashMap<usize, usize> = std::collections::HashMap::new();
        for (_, count) in self.get_available_colors_with_count() {
            *liquid_size_map.entry(count).or_insert(0) += 1;
        }
        for (liquid_size, liquid_count) in liquid_size_map.iter() {
            let container_count = container_size_map.get(liquid_size).unwrap_or(&0);
            if liquid_count > container_count {
                return false;
            }
        }
        true
    }

    fn fast_is_definitely_unsolvable(&self) -> bool {
        // Checks if any liquid cannot possibly fit into any combination of available containers.
        // Does not consider that once a container is used for one color, it can't be used for another.
        // If this returns true, the puzzle is definitely unsolvable. If false, may still be unsolvable.
        // Guaranteed correct if all containers are the same size.
        let containers: Vec<usize> = self
            .fluid_containers
            .iter()
            .map(|c| c.get_capacity())
            .collect();
        let mut reachable_sizes: std::collections::HashSet<usize> = std::collections::HashSet::new();
        reachable_sizes.insert(0);
        for &c in containers.iter() {
            let current_sizes: Vec<usize> = reachable_sizes.iter().copied().collect();
            for &r in current_sizes.iter() {
                reachable_sizes.insert(r + c);
            }
        }
        let liquids: Vec<usize> = self
            .get_available_colors_with_count()
            .iter()
            .map(|(_, count)| *count)
            .collect();
        for liquid_count in liquids.iter() {
            if !reachable_sizes.contains(liquid_count) {
                return true;
            }
        }
        false
    }

    pub fn fast_is_maybe_solvable(&self) -> Option<bool> {
        // Returns Some(true) if definitely solvable, Some(false) if definitely unsolvable, None if unknown
        if self.fast_is_definitely_unsolvable() {
            debug!("Fast definite unsolvability check failed.");
            return Some(false);
        }
        let unique_sizes: std::collections::HashSet<usize> = self.get_container_sizes().iter().copied().collect();
        if unique_sizes.len() == 1 {
            debug!("All containers are the same size therefore fast unsolvability checker is accurate.");
            return Some(true);
        }
        if self.fast_is_definitely_solvable() {
            debug!("Fast definite solvability check passed.");
            return Some(true);
        }
        None
    }

    fn enumerate_subsets_to_target_size(
        size_counts: &[(usize, usize)],
        chosen_so_far: &mut Vec<(usize, usize)>,
        index: usize,
        target_sizes: &std::collections::HashSet<usize>,
        max_size: usize,
        sum_so_far: usize,
        hashmap_to_add_to: &mut std::collections::HashMap<usize, Vec<Vec<(usize, usize)>>>,
    ) {
        if index >= size_counts.len() {
            if target_sizes.contains(&sum_so_far) {
                hashmap_to_add_to.entry(sum_so_far).or_insert_with(Vec::new).push(chosen_so_far.clone());
            }
            return;
        }
        let (value, count) = size_counts[index];
        let base_len = chosen_so_far.len();
        for k in 0..=count {
            let new_sum = sum_so_far + value * k;
            if new_sum > max_size {
                return;
            }
            chosen_so_far.truncate(base_len);
            chosen_so_far.push((value, k));
            Self::enumerate_subsets_to_target_size(size_counts, chosen_so_far, index + 1, target_sizes, max_size, new_sum, hashmap_to_add_to);
        }
    }

    pub fn is_solvable(&self) -> bool {
        // A full check for solvability using recursive subset enumeration
        // If this returns true, there is definitely a way to arrange the liquids that is solved, although it might not be reachable entirely by moves.
        // If false, there is definitely no way to arrange the liquids that is solved.
        // This is a computationally expensive check, so we first run the fast checks.
        if let Some(result) = self.fast_is_maybe_solvable() {
            return result;
        }
        debug!("Proceeding to full solvability check.");
        
        let containers: Vec<usize> = self
            .fluid_containers
            .iter()
            .map(|c| c.get_capacity())
            .collect();
        let mut size_map: std::collections::HashMap<usize, usize> = std::collections::HashMap::new();
        for &c in containers.iter() {
            *size_map.entry(c).or_insert(0) += 1;
        }
        let container_sizes: Vec<(usize, usize)> = size_map.into_iter().collect();
        let liquids: Vec<usize> = self
            .get_available_colors_with_count()
            .iter()
            .map(|(_, count)| *count)
            .collect();
        // liquids.sort();
        let mut ways_to_get_liquids: std::collections::HashMap<usize,Vec<Vec<(usize, usize)>>> = std::collections::HashMap::with_capacity(liquids.len());
        for &liquid in liquids.iter() {
            ways_to_get_liquids.insert(liquid, Vec::new());
        }
        debug!("Preprocessing for solvability containers: {:?}, liquids: {:?}", containers, liquids);
        Self::enumerate_subsets_to_target_size(&container_sizes, &mut Vec::new(), 0, &ways_to_get_liquids.keys().copied().collect(), *containers.iter().max().unwrap_or(&0), 0, &mut ways_to_get_liquids);
        debug!("Solving");
        self.recursive_is_solvable(&container_sizes, &liquids)
    }

    fn recursive_is_solvable(&self, , liquids: &[usize]) -> bool {


        //  Next step: See if there's some dynamic programming way to more efficiently find all combinations of container sizes that sum to a precise target liquid size.
        if liquids.is_empty() {
            debug!("All liquids have been successfully matched.");
            return true;
        }
        // experimental optimization: check if any liquid cannot possibly fit into any combination of available containers.
        // let mut reachable_sizes: std::collections::HashSet<usize> = std::collections::HashSet::new();
        // reachable_sizes.insert(0);
        // for &(size, count) in container_sizes.iter() {
        //     let current_sizes: Vec<usize> = reachable_sizes.iter().copied().collect();
        //     for i in 0..count {
        //         for &r in current_sizes.iter() {
        //             reachable_sizes.insert(r + size*(i+1));
        //         }
        //     }
        // }
        // for liquid_count in liquids.iter() {
        //     if !reachable_sizes.contains(liquid_count) {
        //         return false;
        //     }
        // }
        // end optimization

        let first_liquid = liquids[0];
        if first_liquid >= 38 {
            debug!("Trying to fit liquid of size {}", first_liquid);
        }
        let remaining_liquids = &liquids[1..];

        let mut subset_buffer: Vec<(usize, usize)> = Vec::with_capacity(container_sizes.len());
        let mut found = false;
        Self::enumerate_unique_subsets(container_sizes, 0, &mut subset_buffer, &mut |subset: &[(usize, usize)]| {
            let subset_sum: usize = subset.iter().map(|(v, c)| v * c).sum();
            if subset_sum != first_liquid {return false;}
            
            let mut remaining_containers: Vec<(usize, usize)> = container_sizes.to_vec();
            for (value, count) in remaining_containers.iter_mut() {
                for &(sub_value, sub_count) in subset.iter() {
                    if *value == sub_value {
                        *count = count.saturating_sub(sub_count);
                    }
                }
            }
            remaining_containers.retain(|&(_, c)| c > 0);
            if self.recursive_is_solvable(&remaining_containers, remaining_liquids) {
                found = true;
                return true;
            }
            false
        });

        if found {
            return true;
        }
        false
    }
}

impl PartialEq for GameState {
    fn eq(&self, other: &Self) -> bool {
        self.get_sorted_containers() == other.get_sorted_containers()
    }
}

impl Eq for GameState {}

// Controls / Button

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ControlAction {
    SelectColor(usize),
    SelectContainer(usize),
    Deselect,
    PourInto(usize, usize),
    ReversePour(usize, usize, usize),
    Undo,
    Redo,
    Reset,
    ToggleEditor,
    CopyState,
    // Editor actions
    PasteState,
    AddColor(usize, usize),
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
        matches!(
            self.action,
            ControlAction::AddColor(_, _)
                | ControlAction::RemoveColor(_)
                | ControlAction::AddContainer
                | ControlAction::RemoveContainer
                | ControlAction::ExpandContainer
                | ControlAction::ShrinkContainer
                | ControlAction::PasteState
        )
    }
}

// Hit testing

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum HitItem {
    Button { function: ControlAction },
    Container { index: usize },
    Swatch { index: usize },
    #[allow(dead_code)]
    PacketInContainer {
        container_index: usize,
        packet_index: usize,
    },
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
