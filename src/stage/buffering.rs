use crate::model::*;

use super::cfg::BufferingCfg;
use mincost::{Evolution, EvolutionConfig, Individual};
use std::collections::HashMap;

impl ClockTree {
    pub fn buffering(&mut self, cfg: &BufferingCfg) -> Option<()> {
        let buffer_lib: HashMap<u8, String> = HashMap::new();

        let upper = cfg.buffer_list.len() as u8;
        let evolution_cfg = EvolutionConfig {
            pop_size: cfg.pop_size,
            elite_size: cfg.elite_size,
            mutation_rate: cfg.mutation_rate,
            generations: cfg.generations,
            individual_length: self.tree_level,
            upper: Some(upper),
            lower: Some(0),
        };

        // fitness function in buffering

        // Individal<u8> means each level's insertion result in clock tree, in top-down order
        // Say, if there is totally 3 level in clock tree, then Individal<u8> length = 3
        let fitness = |solution: &Individual<u8>| -> f32 { 0.0 };

        let mut evolution = Evolution::init_with_range(evolution_cfg, fitness).unwrap();
        let final_solution: Individual<u8> = evolution.evolute().unwrap();
        // TODO: check buffer insertion solution equal to clock tree level
        assert!(true);

        let mut current_nodes = vec![];
        let mut next_nodes = vec![];
        let mut buffer_inserted_num: BufferIndex = 0;
        for (level, insertion) in final_solution.genes.iter().enumerate() {
            next_nodes.clear();
            if level == 0 {
                let root_merge: MergeUnitIndex = match self.nodes[self.root_node_index].node_owner {
                    NodeOwner::MergeUnit(midx) => midx,
                    _ => return None,
                };
                if *insertion != 0 {
                    let buffer_model: &str = buffer_lib.get(&insertion).unwrap();
                    self.insert_buffer(buffer_model, buffer_inserted_num, self.root_node_index);
                    buffer_inserted_num += 1;
                }
                next_nodes = self
                    .get_merge_unit_load_nodes(root_merge)
                    .into_iter()
                    .collect();
            } else {
                if *insertion != 0 {
                    let buffer_model: &str = buffer_lib.get(&insertion).unwrap();
                    for nidx in &current_nodes {
                        self.insert_buffer(buffer_model, buffer_inserted_num, *nidx);
                        buffer_inserted_num += 1;
                    }
                }
                for nidx in current_nodes {
                    let root_merge: MergeUnitIndex = match self.nodes[nidx].node_owner {
                        NodeOwner::MergeUnit(midx) => midx,
                        _ => return None,
                    };
                    next_nodes.extend(self.get_merge_unit_load_nodes(root_merge).into_iter());
                }
            }
            current_nodes = next_nodes.clone();
        }
        Some(())
    }
}
