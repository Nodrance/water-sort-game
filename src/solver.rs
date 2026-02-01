use crate::model::*;
use crate::gameplay::*;
use std::collections::HashMap;
use std::collections::HashSet;
use rayon::prelude::*;
use macroquad::prelude::debug;
use rand::prelude::*;

use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

#[derive(Clone)]
struct GameStateWithHistory {
    state: GameState,
    history: Vec<MoveAction>,
}
impl GameStateWithHistory {
    pub fn merge(mut self, other: GameStateWithHistory) -> GameStateWithHistory {
        assert !(self.state == other.state, "Cannot merge different game states");
        let shortest_history = if self.history.len() < other.history.len() {
            self.history.clone()
        } else {
            other.history.clone()
        };
        GameStateWithHistory {
            state: self.state,
            history: shortest_history,
        }
    }
}
impl PartialEq for GameStateWithHistory {
    fn eq(&self, other: &Self) -> bool {
        self.state == other.state
    }
}
impl Eq for GameStateWithHistory {}

pub struct Solver {
    starting_state: GameState,
    considering_states: Vec<GameStateWithHistory>,
    visited_states: Vec<GameStateWithHistory>,
}

impl Solver {
    pub fn new(starting_state: GameState) -> Solver {
        Solver {
            starting_state: starting_state.clone(),
            considering_states: vec![GameStateWithHistory {
                state: starting_state,
                history: vec![],
            }],
            visited_states: vec![],
        }
    }
    fn consider_state(&mut self, state_with_history: GameStateWithHistory) {
        if !self
            .visited_states
            .iter()
            .any(|s| s.state == state_with_history.state)
        {
            self.considering_states.push(state_with_history);
        }
    }
}

impl GameState {
    fn fast_is_definitely_solvable(&self) -> bool {
        // Checks if every liquid can perfectly fit into containers of the same size.
        // If this returns true, the puzzle is definitely solvable. If false, may still be solvable.
        // Guaranteed correct if no liquid ends up split across multiple containers.
        let mut container_size_map: HashMap<usize, usize> = HashMap::new();
        for container in &self.fluid_containers {
            *container_size_map.entry(container.get_capacity()).or_insert(0) += 1;
        }
        let mut liquid_size_map: HashMap<usize, usize> = HashMap::new();
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
        let mut reachable_sizes: HashSet<usize> = HashSet::new();
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
        if !reachable_sizes.contains(&self.get_empty_spaces_count()) {
            // All the empty space must be in containers too
            return true;
        }
        false
    }

    pub fn fast_is_maybe_solvable(&self) -> Option<bool> {
        // Returns Some(true) if definitely solvable, Some(false) if definitely unsolvable, None if unknown
        if self.is_solved() {
            debug!("Game state is already solved.");
            return Some(true);
        }
        if self.fast_is_definitely_unsolvable() {
            debug!("Fast definite unsolvability check failed.");
            return Some(false);
        }
        let unique_sizes: HashSet<usize> = self.get_container_sizes().iter().copied().collect();
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
        container_size_and_count_vec: &Vec<(usize, usize)>,
        chosen_so_far: &mut HashMap<usize, usize>,
        index: usize,
        target_sizes: &HashSet<usize>,
        max_size: usize,
        sum_so_far: usize,
        hashmap_to_add_to: &mut HashMap<usize, Vec<HashMap<usize, usize>>>,
    ) {
        let (value, count) = container_size_and_count_vec[index];
        let map_length = container_size_and_count_vec.len();
        for k in 0..=count {
            let new_sum = sum_so_far + value * k;
            if new_sum > max_size {
                return;
            }
            if index + 1 >= map_length {
                if target_sizes.contains(&new_sum) {
                    chosen_so_far.insert(value, k);
                    hashmap_to_add_to.entry(new_sum).or_default().push(chosen_so_far.clone());
                }
                continue;
            }
            chosen_so_far.insert(value, k);
            Self::enumerate_subsets_to_target_size(container_size_and_count_vec, chosen_so_far, index + 1, target_sizes, max_size, new_sum, hashmap_to_add_to);
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
        debug!("Fast checks inconclusive, proceeding to full solvability check.");
        
        let containers_vec: Vec<usize> = self
            .fluid_containers
            .iter()
            .map(|c| c.get_capacity())
            .collect();
        let mut container_size_to_count_map: HashMap<usize, usize> = HashMap::new();
        for &c in containers_vec.iter() {
            *container_size_to_count_map.entry(c).or_insert(0) += 1;
        }
        let mut container_size_and_count_vec: Vec<(usize, usize)> = container_size_to_count_map.iter().map(|(size, count)| (*size, *count)).collect();
        container_size_and_count_vec.sort_by(|a, b| b.0.cmp(&a.0));

        let mut liquid_size_vec: Vec<usize> = self
            .get_available_colors_with_count()
            .iter()
            .map(|(_, count)| *count)
            .collect();
        if *liquid_size_vec.iter().max().unwrap_or(&0) > self.get_empty_spaces_count() {
            liquid_size_vec.push(self.get_empty_spaces_count());
        }
        let liquid_sizes_set: HashSet<usize> = liquid_size_vec.iter().copied().collect();
        let mut ways_to_get_liquids: HashMap<usize,Vec<HashMap<usize,usize>>> = HashMap::with_capacity(liquid_sizes_set.len());
        for &liquid in liquid_sizes_set.iter() {
            ways_to_get_liquids.insert(liquid, Vec::new());
        }
        let total_iterations = container_size_and_count_vec.iter().map(|(_, count)| *count + 1).product::<usize>();
        debug!("Preprocessing subsets, total subset combinations to consider: {}", total_iterations);
        Self::enumerate_subsets_to_target_size(
            &container_size_and_count_vec,
            &mut HashMap::with_capacity(container_size_to_count_map.len()), 
            0,
            &liquid_sizes_set,
            *liquid_sizes_set.iter().max().unwrap_or(&0),
            0,
            &mut ways_to_get_liquids,
        );

        debug!("Pruning forced options");
        loop {
            if ways_to_get_liquids.values().any(|v| v.is_empty()) {
                if let Some(unsolvable_liquid) = ways_to_get_liquids.iter().find(|(_, v)| v.is_empty()).map(|(k, _)| *k) {
                    debug!("No ways to make a liquid of size {} after pruning, unsolvable.", unsolvable_liquid);
                }
                return false;
            }
            let single_option = ways_to_get_liquids.iter().find(|(_, v)| v.len() == 1);
            if single_option.is_none() {
                break;
            }
            let (liquid_size, ways) = single_option.unwrap();
            let liquid_size = *liquid_size;
            let way = ways[0].clone();
            let liquid_count_using_way  = liquid_size_vec.iter().filter(|&&s| s == liquid_size).count();
            for (key, other_solution_set) in ways_to_get_liquids.iter_mut() {
                if *key == liquid_size {
                    continue;
                }
                other_solution_set.retain(|solution| {
                    for (container_size, used_count) in way.iter() {
                        let entry = solution.get(container_size).unwrap_or(&0);
                        let available_containers = container_size_to_count_map.get(container_size).unwrap_or(&0);
                        if available_containers - *entry < *used_count * liquid_count_using_way {
                            return false;
                        }
                    }
                    true
                });
            }
            for (size, count) in way.iter() {
                let entry = container_size_to_count_map.get_mut(size).unwrap();
                if *count * liquid_count_using_way > *entry {
                    debug!("Not enough containers of size {} to satisfy forced choice, unsolvable.", size);
                    return false;
                }
                *entry -= *count * liquid_count_using_way;
            }
            ways_to_get_liquids.remove(&liquid_size);
            liquid_size_vec.retain(|&s| s != liquid_size);
        }
        debug!("Recursive solving");

        let found = Arc::new(AtomicBool::new(false));
        Self::recursive_is_solvable(
            &ways_to_get_liquids,
            container_size_to_count_map,
            &liquid_size_vec,
            &found,
        )
    }

    fn recursive_is_solvable(
        ways_to_get_liquids: &HashMap<usize, Vec<HashMap<usize, usize>>>,
        remaining_container_sizes: HashMap<usize, usize>,
        liquid_sizes: &[usize],
        found: &Arc<AtomicBool>,
    ) -> bool {
        // If another branch already proved solvable, stop ASAP.
        if found.load(Ordering::Relaxed) {
            return true;
        }

        if liquid_sizes.is_empty() {
            debug!("All liquids have been successfully matched.");
            found.store(true, Ordering::Relaxed);
            return true;
        }

        let current_liquid = liquid_sizes[0];
        let Some(ways) = ways_to_get_liquids.get(&current_liquid) else {
            return false;
        };

        ways.par_iter().any(|way| {
            if found.load(Ordering::Relaxed) {
                return true;
            }

            let mut new_remaining_container_sizes = remaining_container_sizes.clone();

            for (size, count) in way.iter() {
                if found.load(Ordering::Relaxed) {
                    return true;
                }

                let entry = new_remaining_container_sizes.entry(*size).or_insert(0);
                if *entry < *count {
                    return false;
                }
                *entry -= *count;
            }

            Self::recursive_is_solvable(
                ways_to_get_liquids,
                new_remaining_container_sizes,
                &liquid_sizes[1..],
                found,
            )
        })
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
                    if amount == from_container.get_top_fluid_depth() {
                        // If the amount is less, then this move is reversible, so we only consider full pours to reduce the search space
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

    pub fn get_possible_reverse_moves(&self, limit_size: bool) -> Vec<MoveAction> {
        let mut moves = vec![];
        for (from_index, from_container) in self.fluid_containers.iter().enumerate() {
            if from_container.is_empty() {
                continue;
            }
            for (to_index, to_container) in self.fluid_containers.iter().enumerate() {
                if from_container == to_container || from_container.get_top_fluid() == to_container.get_top_fluid() {
                    // If the top fluids are the same, then the move is reversible, so we skip it to reduce the search space
                    continue;
                }
                let mut amount = from_container.get_reverse_pourable_amount(to_container);
                if limit_size && from_container.get_filled_amount() == amount {
                    amount -= 1;
                }
                if amount > 0 {
                    moves.push(MoveAction {
                        from_container: from_index,
                        to_container: to_index,
                        amount,
                    });
                }
            }
        }
        moves
    }

    pub fn apply_random_move(&mut self) -> bool {
        let possible_moves = self.get_possible_moves();
        if possible_moves.is_empty() {
            return false;
        }
        let mut rng = rand::rng();
        let selected_move = &possible_moves.choose(&mut rng).unwrap();
        self.apply_move(selected_move);
        true
    }
    pub fn apply_random_reverse_move(&mut self) -> bool {
        let possible_moves = self.get_possible_reverse_moves(true);
        if possible_moves.is_empty() {
            return false;
        }
        let mut rng = rand::rng();
        let selected_move = &possible_moves.choose(&mut rng).unwrap();
        self.apply_reverse_move(selected_move);
        true
    }

    pub fn shuffle(&mut self) {
        let mut rng = rand::rng();
        for _ in 0..1000 {
            let mut reverse_moves = self.get_possible_reverse_moves(true);
            let smallest_block_depth = self.fluid_containers
                .iter()
                .map(|container| container.get_top_fluid_depth())
                .min();
            let smallest_block_container_indices: Option<Vec<usize>> = smallest_block_depth.map(|min_depth| {
                self.fluid_containers
                    .iter()
                    .enumerate()
                    .filter(|(_, container)| container.get_top_fluid_depth() == min_depth)
                    .map(|(index, _)| index)
                    .collect()
            });
            if let Some(smallest_indices) = smallest_block_container_indices {
                reverse_moves.retain(|m| smallest_indices.contains(&m.to_container));
            }
            let largest_block_depth = self.fluid_containers
                .iter()
                .map(|container| container.get_top_fluid_depth())
                .max();
            let largest_block_container_indices: Option<Vec<usize>> = largest_block_depth.map(|max_depth| {
                self.fluid_containers
                    .iter()
                    .enumerate()
                    .filter(|(_, container)| container.get_top_fluid_depth() == max_depth)
                    .map(|(index, _)| index)
                    .collect()
            });
            if let Some(largest_indices) = largest_block_container_indices {
                reverse_moves.retain(|m| largest_indices.contains(&m.from_container));
            }

            let selected_move = reverse_moves.choose(&mut rng);
            if let Some(mv) = selected_move {
                self.apply_reverse_move(mv);
            } else {
                break;
            }
        }
    }
}