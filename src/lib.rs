#![allow(dead_code)]
use cts_plugin::{CTSPluginRes, DesignPlugin, PdkPlugin};
use std::collections::{HashMap, HashSet};

use std::fs;
use std::path::Path;

use libloading::Library;
use rand::Rng;

mod merge;
use merge::MergeUnit;

enum MyError {
    IOErr,
    BNPErr,
    DivisionErr,
}

type MyRes<T> = Result<T, MyError>;

#[derive(Default)]
pub struct ClockTree {
    name: String,
    x_range: (i32, i32),
    y_range: (i32, i32),
    sinks: Vec<Sink>,
    buffers: Vec<Buffer>,
    merge: Vec<MergeUnit>,
}

impl ClockTree {
    pub fn new(name: &str) -> Self {
        ClockTree {
            name: name.to_string(),
            ..Default::default()
        }
    }

    pub fn run(&mut self, file: &str) -> CTSPluginRes<()> {
        // prepare via map and layer map
        let pdk_lib = Library::new("/tmp/libvulcan_pdb.so").expect("load library");
        let new_pdk_plugin: libloading::Symbol<fn() -> Box<dyn PdkPlugin>> =
            unsafe { pdk_lib.get(b"new_pdk_plugin") }.expect("load symbol");
        println!("Load vulcan-pdb so successfully");
        let mut my_pdk = new_pdk_plugin();
        // login with username and password
        // !!! first replace them with your username and password
        my_pdk.login("erihsu", "xzy101469*");
        let layer_map: Vec<(i16, String)> = my_pdk.get_layer_map()?;
        let via_map: Vec<(i16, String)> = my_pdk.get_via_map()?;

        // prepare data from def
        let cts_lib = Library::new("/tmp/libvulcan.so").expect("load library");
        let new_design_plugin: libloading::Symbol<fn() -> Box<dyn DesignPlugin>> =
            unsafe { cts_lib.get(b"new_design_plugin") }.expect("load symbol");
        let adjust_pin_location: libloading::Symbol<fn(i32, i32, i8) -> (i32, i32)> =
            unsafe { cts_lib.get(b"adjust_pin_location") }.expect("load symbol");
        println!("Load vulcan so successfully");
        let mut my_design = new_design_plugin();
        // step 1 : prepare via map and layer map before import def
        let lmap: Vec<(String, i16)> = layer_map.iter().map(|x| (x.1.clone(), x.0)).collect();
        let vmap: Vec<(String, i16)> = via_map.iter().map(|x| (x.1.clone(), x.0)).collect();
        let _ = my_design.prepare_layer_map(lmap)?;
        let _ = my_design.prepare_via_map(vmap)?;
        my_design.import_def(file)?;

        println!("Read def successfully");
        let sinks: Vec<(String, (i32, i32), i8)> = my_design.get_clock_sinks(&self.name)?;
        println!("Get {} sinks successfully", sinks.len());
        let mut sink_type = HashSet::new();
        sinks.iter().for_each(|x| {
            sink_type.insert(x.0.to_string());
        });

        let mut sink_offset: HashMap<String, (f32, f32)> = HashMap::new();
        let mut sink_cap: HashMap<String, f32> = HashMap::new();
        let dbu_factor = my_design.get_length_dbu()?;
        println!("Get dbu successfully");
        let dbu_factor = dbu_factor as f32;
        for d in &sink_type {
            println!("cell {}", d);
            let offset = my_pdk.get_sink_clk_pin_offset(d)?;
            println!("get offset {:?}", offset);
            let cap = my_pdk.get_sink_cap(d)?;
            println!("get cap {}", cap);
            sink_offset.insert(d.to_string(), offset);
            sink_cap.insert(d.to_string(), cap);
        }
        println!("Get sink actual location and load pin successfully");
        let (mut x_min, mut x_max, mut y_min, mut y_max) = (i32::MAX, i32::MIN, i32::MAX, i32::MIN);
        self.sinks = sinks
            .iter()
            .map(|x| {
                let orient = x.2;
                let (origin_x, origin_y) = x.1;
                let (offset_x, offset_y) = *sink_offset.get(&x.0).unwrap();
                let offset_dbu = (
                    (dbu_factor * offset_x) as i32,
                    (dbu_factor * offset_y) as i32,
                );
                let adjust_offset_dbu = adjust_pin_location(offset_dbu.0, offset_dbu.1, orient);
                let load_cap = *sink_cap.get(&x.0).unwrap();
                let sink_x = origin_x + adjust_offset_dbu.0;
                let sink_y = origin_y + adjust_offset_dbu.1;
                if sink_x < x_min {
                    x_min = sink_x;
                }
                if sink_x > x_max {
                    x_max = sink_x;
                }
                if sink_y < y_min {
                    y_min = sink_y;
                }
                if sink_y > y_max {
                    y_max = sink_y;
                }
                Sink {
                    name: x.0.to_string(),
                    x: sink_x,
                    y: sink_y,
                    load_cap,
                }
            })
            .collect();
        self.x_range = (x_min, x_max);
        self.y_range = (y_min, y_max);
        println!("Load CTS related data successfully");
        // generate topology
        self.gen_topology();

        // my_design.

        // export def

        Ok(())
    }
    fn gen_topology(&mut self) {
        println!("Start generate topology");
        // step 1: bnp and insert pseudo sink
        let mut branchs = vec![];
        let mut n = self.sinks.len() as u32;
        while n > 1 {
            for i in 2..=n {
                if n % i == 0 {
                    n /= i;
                    branchs.push(i as u32);
                    break;
                }
            }
        }
        println!("branch number planning finished,result:{:?}", branchs);
        // add pseudo sinks with zero capload
        let target_num = branchs.iter().fold(1, |acc, x| acc * x);
        let pseudo_sink = target_num - self.sinks.len() as u32;
        println!("need {} pseudo sink into real sink topo", pseudo_sink);
        if pseudo_sink != 0 {
            let mut rng = rand::thread_rng();
            for _ in 0..pseudo_sink {
                self.sinks.push(Sink {
                    x: rng.gen_range(self.x_range.0..self.x_range.1),
                    y: rng.gen_range(self.y_range.0..self.y_range.1),
                    ..Default::default()
                })
            }
        }
        // step 2 : top-down parition
        let coords: Vec<(i32, i32)> = self.sinks.iter().map(|s| (s.x, s.y)).collect();

        // group label and sink indexing pair. group label can be used later with branchs to fully
        // specify symmetry clock tree topology
        // As a result, the grp label is in increased-order
        let grp2id: Vec<(u32, usize)> = group(&coords, &branchs);

        // step 3: bottom-up merge

        // reverse branchs to bottom-up order
        // hashmap to store map between merge level and target wirelength
        let mut branch2length: HashMap<u8, u32> = HashMap::new();
        branchs.reverse();
        let mut childs: Vec<(i32, i32)> = grp2id
            .iter()
            .map(|d| {
                let s = &self.sinks[d.1];
                (s.x, s.y)
            })
            .collect();
        let mut new_childs: Vec<(i32, i32)> = Vec::new();
        for (level, b) in branchs.iter().enumerate() {
            let level = level as u8;
            let mut target_len = u32::MIN;
            let mut one_merge_childs = Vec::new();
            for (i, s) in childs.iter().enumerate() {
                let i = i as u32;
                if i % *b == 0 && i != 0 {
                    let mut one_merge_inst = MergeUnit::new();
                    one_merge_inst.load_sink(&one_merge_childs);
                    // compare between target length
                    if one_merge_inst.range_length() > target_len {
                        target_len = one_merge_inst.range_length();
                    }
                    one_merge_inst.set_level(level);
                    new_childs.push(one_merge_inst.root.clone());
                    self.merge.push(one_merge_inst);
                    // reset next iter childs
                    one_merge_childs.clear();
                }
                one_merge_childs.push(*s);
            }
            branch2length.insert(level, target_len);
            childs.clear();
            childs = new_childs.clone();
        }
        println!("{:?}", branch2length);
        for m in self.merge.iter_mut() {
            let level = m.level;
            let target_len = *branch2length.get(&level).unwrap();
            // mannually precision
            if (target_len - m.range_length()) > 100 {
                // need adjust merge range
                m.adjust_range(target_len);
                m.merge();
            }
        }
        let total_estimate_wire = self.merge.iter().fold(0, |acc, x| acc + x.length());
        println!(
            "pre-merge finished, estimated wirelength:{}",
            total_estimate_wire
        );
    }
}

#[derive(Default, Debug)]
pub struct Sink {
    name: String, // cell name
    x: i32,
    y: i32,
    load_cap: f32,
}

// // reference https://joshondesign.com/2020/04/08/rust5_tree
// pub struct TreeNode {
//     pub child: RefCell<Vec<Rc<TreeNode>>>,
//     parent: Option<Rc<Weak<TreeNode>>>,
//     // Three types can be hold in node_data: Clock buffer(ViComponent), Sink(ViComponent),Tappoint(VdbDot)
//     node_data: NodeData,

//     // Sym-CTS related
//     radius: f32, // max length in the region
//     fanout: i32,
//     node_cap: f32, // total load cap
//     wires: Vec<Wire>,
// }

// pub enum NodeData {
//     Sink(Box<Sink>),
//     Buffer(Box<Buffer>),
//     Dot(VdbDot),
// }

pub struct Buffer {
    buffer_name: String,
    model_name: String,
    in_pin: BufInPin,
    out_pin: BufOutPin,
}

pub struct BufInPin {
    name: String,
    location: (i32, i32),
    cap: f32,
}

pub struct BufOutPin {
    name: String,
    location: (i32, i32),
}

// get geometry center
fn get_center(coords: &[(i32, i32)]) -> (i32, i32) {
    let mut sum_x: i64 = 0;
    let mut sum_y: i64 = 0;
    let mut n: i64 = 0;
    coords.iter().for_each(|x| {
        n += 1;
        sum_x += x.0 as i64;
        sum_y += x.1 as i64
    });
    let center_x = (sum_x / n) as i32;
    let center_y = (sum_y / n) as i32;
    (center_x, center_y)
}

// to simplify get max distance between two farest point, just use perimeter to represent max
// distance
fn get_max_distance(coords: &[(i32, i32)]) -> u32 {
    let (mut x_min,mut x_max,mut y_min,mut y_max) = (i32::MAX,i32::MIN,i32::MAX,i32::MIN);
    for d in coords {
        if d.0 < x_min {
            x_min = d.0;
        }
        if d.0 > x_max {
            x_max = d.0;
        }
        if d.1 < y_min {
            y_min = d.1;
        }
        if d.1 > y_max {
            y_max = d.1;
        }
    }
    ((x_max - x_min) + (y_max - y_min)) as u32
    // let mut dis = 0;
    // let len = coords.len();
    // for i in 0..len - 1 {
    //     dis += (coords[i].0 - coords[i].1).abs() + (coords[i + 1].1 - coords[i + 1].1).abs();
    // }
    // dis += (coords[len - 1].0 - coords[0].0).abs() + (coords[len - 1].1 - coords[0].1).abs();
    // dis as u32
}

fn group(coords: &[(i32, i32)], branchs: &Vec<u32>) -> Vec<(u32, usize)> {
    let mut result = Vec::new();
    let mut grps = Vec::new();
    for (i, b) in branchs.iter().enumerate() {
        if i == 0 {
            let idxs = (0..coords.len()).collect();
            grps = find_group(coords, (0, idxs), *b);
        } else if i < branchs.len() - 1 {
            let mut new_d = Vec::new();
            for d in &grps {
                let next_grps = find_group(coords, d.clone(), *b);

                for g in next_grps {
                    new_d.push(g);
                }
            }
            grps = new_d;
        } else {
            for d in &grps {
                let next_grps = find_group(coords, d.clone(), *b);
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
    grp2id: (u32, Vec<usize>),
    grpn_next: u32,
) -> Vec<(u32, Vec<usize>)> {
    let mut grp_id_next = vec![];
    let start_gid_next = (grpn_next - 1) * grp2id.0;
    for i in 0..grpn_next {
        grp_id_next.push(start_gid_next + i);
    }
    let locations: Vec<(i32, i32)> = grp2id.1.iter().map(|x| coords[*x]).collect();
    let center = get_center(&locations);
    let relative_location_phase: Vec<i32> = locations
        .iter()
        .map(|d| {
            let x = (d.0 - center.0) as f32;
            let y = (d.1 - center.1) as f32;
            (y.atan2(x) * 10000.0) as i32
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
    let mut min_dis = u32::MAX;
    let mut min_dis_cut = 0;
    // find min_dis_cut that achieve minimal parition cost
    for start_cut in 0..cut_step {
        let mut max_dis = 0;
        for i in 0..grpn_next {
            let mut idxs: Vec<usize> = Vec::new();
            if i == grpn_next - 1 {
                let start: usize = (i * cut_step + start_cut) as usize;
                idxs.extend_from_slice(&sorted_idx[start..]);
                let end: usize = cut_step as usize;
                idxs.extend_from_slice(&sorted_idx[..end]);
            } else {
                let start = (i * cut_step + start_cut) as usize;
                let end = ((i + 1) * cut_step + start_cut) as usize;
                idxs.extend_from_slice(&sorted_idx[start..end]);
            };
            let locs: Vec<(i32, i32)> = idxs.iter().map(|x| coords[*x]).collect();
            let dis = get_max_distance(&locs);
            if dis > max_dis {
                max_dis = dis;
            }
        }
        if max_dis < min_dis {
            min_dis_cut = start_cut;
            min_dis = max_dis;
        }
    }
    let mut result = Vec::new();
    for (i, gid) in grp_id_next.iter().enumerate() {
        let i = i as u32;
        let mut idxs: Vec<usize> = Vec::new();
        if i == grpn_next - 1 {
            let start = (i * cut_step + min_dis_cut) as usize;
            idxs.extend_from_slice(&sorted_idx[start..]);
            let end = (cut_step) as usize;
            idxs.extend_from_slice(&sorted_idx[0..end]);
        } else {
            let start = (i * cut_step + min_dis_cut) as usize;
            let end = ((i + 1) * cut_step + min_dis_cut) as usize;
            idxs.extend_from_slice(&sorted_idx[start..end]);
        };
        result.push((*gid, idxs))
    }
    result
}

// fn buffering(topology: &mut [TreeNode]) -> MyRes<()> {
//     Ok(())
// }
