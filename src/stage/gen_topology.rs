use super::super::model::*;
use super::cfg::GenTopologyCfg;
use core::f32::consts::PI;
use rand::Rng;
type GroupLabel = u32;

impl ClockTree {
    /// Two things happened in this stage
    ///
    /// 1. After BNP(branch number planning), fanout_map is updated.
    ///    If there needs to insert pseudo sink, then ClockTree's sink field is updated
    /// 2. After Tree construction, ClockTree's merges , nodes and fanout_map field are first time updated

    pub fn gen_topology(&mut self, cfg: &GenTopologyCfg) {
        let mut branchs = vec![];
        let mut n = self.sinks.len();
        while n > 1 {
            for i in 2..=cfg.max_branch {
                if n % i == 0 {
                    n /= i;
                    branchs.push(i as u32);
                    break;
                } else if i == cfg.max_branch {
                    n += 1;
                }
            }
        }
        for (i, b) in branchs.iter().enumerate() {
            self.fanout_map.insert(i, *b as u32);
        }
        self.tree_level = branchs.len() as ClockTreeLevel;
        let target_num = branchs.iter().fold(1, |acc, x| acc * x) as usize;
        let pseudo_sink = target_num - self.sinks.len();
        if pseudo_sink != 0 {
            let mut rng = rand::thread_rng();
            for _ in 0..pseudo_sink {
                let x = rng.gen_range(self.x_range.0..self.x_range.1);
                let y = rng.gen_range(self.y_range.0..self.y_range.1);
                self.sinks.push(Sink {
                    name: String::from(""), // empty name
                    location: (x, y),
                })
            }
        }
        let coords: Vec<Location> = self.sinks.iter().map(|s| s.location).collect();
        let grp2id: Vec<(GroupLabel, SinkIndex)> = group(&coords, &branchs);

        let mut childs: Vec<SinkIndex> = grp2id.iter().map(|d| d.1).collect();

        // store roots location of nodes in same level when merging
        let mut new_childs: Vec<MergeUnitIndex> = Vec::new();

        // reverse branchs to bottom-up order
        // hashmap to store map between merge level and target wirelength
        branchs.reverse();
        let mut nidx: NodeIndex = 0;
        let mut midx: MergeUnitIndex = 0;
        let mut one_merge_child_location: Vec<Location> = Vec::new();
        // construct merge unit
        for (i, b) in branchs.iter().enumerate() {
            let level = self.tree_level - i;
            let mut target_len = u32::MIN;
            if i == 0 {
                for (j, s) in childs.iter().enumerate() {
                    let n = j as u32;
                    one_merge_child_location.push(self.sinks[*s].location);
                    if (n + 1) & *b == 0 {
                        let mut one_merge_inst = MergeUnit {
                            first_node: nidx,
                            ..Default::default()
                        };
                        self.nodes.push(Node {
                            node_owner: NodeOwner::Sink(*s),
                            next_node: None,
                        });
                        nidx += 1;
                        one_merge_inst.analyze_child_location(&one_merge_child_location);
                        one_merge_child_location.clear();
                        // one_merge_inst.set_level(level);
                        // get target length in current tree level by comparing bewteen same level of MergeUnit
                        if one_merge_inst.common_length > target_len {
                            target_len = one_merge_inst.common_length;
                        }
                        new_childs.push(midx);
                        self.merges.push(one_merge_inst);
                        midx += 1;
                    } else {
                        self.nodes.push(Node {
                            node_owner: NodeOwner::Sink(*s),
                            next_node: Some(nidx + 1),
                        });
                        nidx += 1;
                    }
                }
            } else {
                for (j, s) in childs.iter().enumerate() {
                    let n = j as u32;
                    one_merge_child_location.push(self.sinks[*s].location);
                    if (n + 1) & *b == 0 {
                        let mut one_merge_inst = MergeUnit {
                            first_node: nidx,
                            ..Default::default()
                        };
                        self.nodes.push(Node {
                            node_owner: NodeOwner::MergeUnit(*s),
                            next_node: None,
                        });
                        nidx += 1;
                        one_merge_inst.analyze_child_location(&one_merge_child_location);
                        one_merge_child_location.clear();
                        // one_merge_inst.set_level(level);
                        // get target length in current tree level by comparing bewteen same level of MergeUnit
                        if one_merge_inst.common_length > target_len {
                            target_len = one_merge_inst.common_length;
                        }
                        new_childs.push(midx);
                        self.merges.push(one_merge_inst);
                        midx += 1;
                    } else {
                        self.nodes.push(Node {
                            node_owner: NodeOwner::MergeUnit(*s),
                            next_node: Some(nidx + 1),
                        });
                        nidx += 1;
                    }
                }
            }
            self.length_map.insert(level, target_len);

            childs = new_childs.clone();
            new_childs.clear();
        }

        let mut total_estimate_wire = 0;
        let mut fanout_mul = 1;
        for level in 0..self.tree_level {
            fanout_mul *= self.fanout_map.get(&level).unwrap();
            total_estimate_wire += self.length_map.get(&level).unwrap() * fanout_mul;
        }

        println!(
            "pre-merge finished, estimated wirelength:{}",
            total_estimate_wire
        );
    }
}

fn group(coords: &[(i32, i32)], branchs: &Vec<u32>) -> Vec<(u32, usize)> {
    let mut result = Vec::new();
    let mut grps = Vec::new();
    let center = get_center(coords);
    for (i, b) in branchs.iter().enumerate() {
        if i == 0 {
            let idxs = (0..coords.len()).collect();
            grps = find_group(coords, center, (0, idxs), *b);
        } else if i < branchs.len() - 1 {
            let mut new_d = Vec::new();
            for d in &grps {
                let next_grps = find_group(coords, center, d.clone(), *b);

                for g in next_grps {
                    new_d.push(g);
                }
            }
            grps = new_d;
        } else {
            for d in &grps {
                let next_grps = find_group(coords, center, d.clone(), *b);
                for v in next_grps {
                    // Vec<(u32,Vec<usize>)> become Vec<(u32,vec![usize])>
                    result.push((v.0, v.1[0]));
                }
            }
        }
    }
    result
}

fn find_group(
    coords: &[(i32, i32)],
    center: (i32, i32),
    grp2id: (u32, Vec<usize>),
    grpn_next: u32,
) -> Vec<(u32, Vec<usize>)> {
    let mut grp_id_next = vec![];
    let start_gid_next = (grpn_next - 1) * grp2id.0;
    for i in 0..grpn_next {
        grp_id_next.push(start_gid_next + i);
    }
    let locations: Vec<(i32, i32)> = grp2id.1.iter().map(|x| coords[*x]).collect();
    let relative_location_phase: Vec<i32> = locations
        .iter()
        .map(|d| {
            let x = (d.0 - center.0) as f32;
            let y = (d.1 - center.1) as f32;
            let phase: f32 = if y > 0.0 {
                y.atan2(x)
            } else {
                y.atan2(x) + 2.0 * PI
            };
            (phase * 10000.0) as i32
        })
        .collect();
    let mut pre_sort_data: Vec<(&usize, &i32)> = grp2id
        .1
        .iter()
        .zip(&mut relative_location_phase.iter())
        .collect();
    pre_sort_data.sort_by(|a, b| a.1.cmp(&b.1));
    let sorted_idx: Vec<usize> = pre_sort_data.iter().map(|x| x.0.clone()).collect();
    let cut_step = sorted_idx.len() as u32 / grpn_next;
    let mut result = Vec::new();
    for (i, gid) in grp_id_next.iter().enumerate() {
        let i = i as u32;
        let start = (cut_step * i) as usize;
        let end = (cut_step * (i + 1)) as usize;
        let idxs: Vec<usize> = (&sorted_idx[start..end]).to_vec();

        result.push((*gid, idxs))
    }
    result
}

fn get_center(coords: &[(i32, i32)]) -> (i32, i32) {
    let mut n = 0;
    let mut x: i64 = 0;
    let mut y: i64 = 0;
    for d in coords {
        n += 1;
        x += d.0 as i64;
        y += d.1 as i64;
    }
    ((x / n) as i32, (y / n) as i32)
}
