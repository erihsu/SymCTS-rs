use cts_plugin::Path;
use std::collections::HashMap;

#[derive(Default)]
pub struct ClockTree {
    pub name: String,
    pub x_range: (i32, i32),
    pub y_range: (i32, i32),
    pub nodes: Vec<Node>,
    pub tree_level: ClockTreeLevel,
    pub root_node_index: NodeIndex,
    pub sinks: Vec<Sink>,
    pub buffers: Vec<Buffer>,
    pub merges: Vec<MergeUnit>,
    // after gen_topology stage, length_map & fanout_map is generated
    pub length_map: HashMap<ClockTreeLevel, u32>, // <level, common length> mapping
    pub fanout_map: HashMap<ClockTreeLevel, u32>, // <level, fanout> mapping
}

impl ClockTree {
    pub fn get_merge_unit_load_nodes(&self, idx: MergeUnitIndex) -> NodeSuccessor {
        NodeSuccessor {
            tree: self,
            current_node_idx: self.merges[idx].first_node,
        }
    }
    pub fn get_buffer_load_nodes(&self, idx: BufferIndex) -> NodeSuccessor {
        let merge_unit_load = self.buffers[idx].load;
        self.get_merge_unit_load_nodes(merge_unit_load)
    }
    pub fn insert_buffer(
        &mut self,
        buffer_model: &str,
        buffer_idx: BufferIndex,
        node_idx: NodeIndex,
    ) -> Option<()> {
        let mut node = &mut self.nodes[node_idx];
        match node.node_owner {
            NodeOwner::MergeUnit(d) => {
                let new_buffer = Buffer {
                    buffer_name: format!("{}_{}", buffer_model, node_idx as u32),
                    model_name: buffer_model.to_string(),
                    location: self.merges[d].location,
                    load: d, // load
                };
                self.buffers.push(new_buffer);
                node.node_owner = NodeOwner::Buffer(buffer_idx);
            }
            _ => return None,
        };

        Some(())
    }
}

pub type MergeUnitIndex = usize;
pub type ClockTreeLevel = usize;
pub type SinkIndex = usize;
pub type BufferIndex = usize;
pub type NodeIndex = usize;
pub type Location = (i32, i32);

#[derive(Default)]
pub struct MergeUnit {
    pub first_node: NodeIndex, // child node
    pub location: Location,    // root location
    pub common_length: u32,
    pub path: Vec<Path>,
    pub if_horizontal: bool, // merge unit direction
}

impl MergeUnit {
    pub fn length(&self) -> i32 {
        self.path.iter().fold(0, |acc, x| acc + x.length())
    }

    // load sink , analyze range, get root and common length
    pub fn analyze_child_location(&mut self, sink: &[(i32, i32)]) {
        let (mut x_min, mut x_max, mut y_min, mut y_max) = (i32::MAX, i32::MIN, i32::MAX, i32::MIN);
        let mut lm = (0, 0);
        let mut rm = (0, 0);
        let mut tm = (0, 0);
        let mut bm = (0, 0);
        sink.iter().for_each(|d| {
            // most left
            if d.0 < x_min {
                x_min = d.0;
                lm = *d;
            }
            // most right
            if d.0 > x_max {
                x_max = d.0;
                rm = *d;
            }
            // most bottom
            if d.1 < y_min {
                y_min = d.1;
                bm = *d;
            }
            // most top
            if d.1 > y_max {
                y_max = d.1;
                tm = *d;
            }
        });

        // analyze
        if (x_min - x_max).abs() > (y_min - y_max).abs() {
            self.if_horizontal = true;
            self.location = ((lm.0 + rm.0) / 2, (lm.1 + rm.1) / 2) as Location;
            self.common_length =
                ((lm.0 - self.location.0).abs() + (lm.1 - self.location.1).abs()) as u32;
        } else {
            self.if_horizontal = false;
            self.location = ((bm.0 + tm.0) / 2, (bm.1 + tm.1) / 2) as Location;
            self.common_length =
                ((bm.0 - self.location.0).abs() + (bm.1 - self.location.1).abs()) as u32;
        }
    }
}

use std::iter::Iterator;

pub enum NodeOwner {
    Buffer(BufferIndex),
    MergeUnit(MergeUnitIndex),
    Sink(SinkIndex),
}

pub struct Node {
    pub node_owner: NodeOwner,
    pub next_node: Option<NodeIndex>, // next node belongs to the same parent
}

pub struct NodeSuccessor<'a> {
    tree: &'a ClockTree,
    current_node_idx: NodeIndex,
}

impl<'a> Iterator for NodeSuccessor<'a> {
    type Item = NodeIndex;
    fn next(&mut self) -> Option<NodeIndex> {
        match self.tree.nodes[self.current_node_idx].next_node {
            None => None,
            Some(idx) => {
                self.current_node_idx = idx;
                Some(idx)
            }
        }
    }
}

#[derive(Default, Debug)]
pub struct Sink {
    pub name: String, // cell name
    pub location: Location,
}

pub struct Buffer {
    pub buffer_name: String,
    pub model_name: String,
    pub location: Location,
    pub load: MergeUnitIndex, // load
}

// two dimensional look up table
pub struct LutModel {
    index_1: Vec<f32>,
    index_2: Vec<f32>,
    value: Vec<Vec<f32>>,
}

impl LutModel {
    fn init_from_json(value:) -> Self {

    }

    fn get_value(index_1:f32,index_2:f32) -> f32 {

    }
}