#![allow(dead_code)]

use cts_plugin::{CTSPluginRes, DesignPlugin, PdkPlugin};
use libloading::Library;
use rand::Rng;
use serde_json::Value;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::f32::consts::PI;
use std::rc::Rc;

mod merge;
mod node;
use merge::MergeUnit;
use node::TreeNode;

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
    root: Option<RefCell<Rc<TreeNode>>>,
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

        let mut sink_offset: HashMap<String, HashMap<i8, (f32, f32)>> = HashMap::new();
        let mut sink_cap: HashMap<String, f32> = HashMap::new();
        let dbu_factor = my_design.get_length_dbu()?;
        println!("Get dbu successfully");
        let dbu_factor = dbu_factor as f32;
        for d in &sink_type {
            println!("cell {}", d);
            let offset: HashMap<i8, (f32, f32)> = my_pdk.get_sink_clk_pin_offset(d)?;
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
                let (offset_x, offset_y) =
                    *sink_offset.get(&x.0).and_then(|x| x.get(&orient)).unwrap();
                let offset_dbu = (
                    (dbu_factor * offset_x) as i32,
                    (dbu_factor * offset_y) as i32,
                );
                // let load_cap = *sink_cap.get(&x.0).unwrap();
                let sink_x = origin_x + offset_dbu.0;
                let sink_y = origin_y + offset_dbu.1;
                // let sink_x = origin_x;
                // let sink_y = origin_y;
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
                    load_cap: 0.0,
                }
            })
            .collect();
        self.x_range = (x_min, x_max);
        self.y_range = (y_min, y_max);
        println!("x_range:{:?}", self.x_range);
        println!("y_range:{:?}", self.y_range);
        println!("Load CTS related data successfully");
        // generate topology
        let level2length: HashMap<u8, u32> = self.gen_topology();

        // prepare buffer related data
        let buffers: HashMap<String, Value> = HashMap::new();
        let buffer_names: Vec<String> = my_pdk.list_all_clock_buffer()?;
        let clock_buffer_names: Vec<String> = buffer_names
            .iter()
            .filter(|x| x.contains("BUFH"))
            .map(|x| x.to_string())
            .collect();
        println!("{:?}", clock_buffer_names);
        println!("Totally got {} from pdk", clock_buffer_names.len());

        // for d in &clock_buffer_names {
        //     let a_buffer = my_pdk.get_buffer(&d)?;
        //     buffers.insert(d.to_string(),a_buffer);
        // }
        // my_design.
        self.buffering(&level2length, &buffers);

        // edit design. Including add buffer and all clock path into design

        // add buffer
        for d in &self.buffers {
            // always orient N buffer
            my_design.add_clock_buffer(&d.model_name, d.location, 0)?;
        }

        // add net
        let mut clock_net: Vec<cts_plugin::Route> = Vec::new();
        // let clk_pin = my_design.get_clk_pin(&self.name)?;
        let mut root = (0, 0);
        let clk_pin = (25235,28910);
        for d in &self.merge {
            for p in &d.path {
                let mut element: Vec<cts_plugin::Element> = Vec::new();
                if !p.if_turn() {
                    let p = cts_plugin::Path {
                        from: p.from,
                        to: p.to,
                    };
                    // println!("x=[{},{}]",&p.from.0,&p.to.0);
                    // println!("y=[{},{}]",&p.from.1,&p.to.1);
                    // println!("plt.plot(x,y)");
                    element.push(cts_plugin::Element::Path(p));
                } else {
                    let turn_point = p.turn.unwrap();
                    let p1 = cts_plugin::Path {
                        from: p.from,
                        to: turn_point,
                    };
                    let p2 = cts_plugin::Path {
                        from: turn_point,
                        to: p.to,
                    };
                    // println!("x=[{},{},{}]",p.from.0,turn_point.0,p.to.0);
                    // println!("y=[{},{},{}]",p.from.1,turn_point.1,p.to.1);
                    // println!("plt.plot(x,y)");
                    element.push(cts_plugin::Element::Path(p1));
                    element.push(cts_plugin::Element::Path(p2));
                }
                clock_net.push(cts_plugin::Route {
                    layer: "M6",
                    element: element,
                });
                if d.is_root() {
                    root = d.root;
                }
            }
        }
        let mut element: Vec<cts_plugin::Element> = Vec::new();
        let turn_point_x = ((root.0 + clk_pin.0) / 2) as i32;
        let p1 = cts_plugin::Path {
            from: root,
            to: (turn_point_x, root.1),
        };
        let p2 = cts_plugin::Path {
            from: (turn_point_x, root.1),
            to: (turn_point_x, clk_pin.1),
        };
        let p3 = cts_plugin::Path {
            from: (turn_point_x, clk_pin.1),
            to: clk_pin,
        };
        // println!("x=[{},{},{},{}]",root.0,turn_point_x,turn_point_x,clk_pin.0);
        // println!("y=[{},{},{},{}]",root.1,root.1,clk_pin.1,clk_pin.1);
        // println!("plt.plot(x,y)");
        element.push(cts_plugin::Element::Path(p1));
        element.push(cts_plugin::Element::Path(p2));
        element.push(cts_plugin::Element::Path(p3));
        clock_net.push(cts_plugin::Route {
            layer: "M6",
            element: element,
        });
        my_design.add_clock_net(&self.name, &clock_net)?;
        // export def
        my_design.export_def("exported.def")?;
        Ok(())
    }

    // gen_topology generate a full un-buffered symmetric clock tree that represent by sets of MergeUnit
    //
    // return:HashMap<tree_level,target_length>
    fn gen_topology(&mut self) -> HashMap<u8, u32> {
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
        branchs = vec![2,2,2,3,3,13];
        println!("branch number planning finished,result:{:?}", branchs);
        let max_level = branchs.len() as u8; // not larger than 128
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
        // STEP 2 : top-down parition
        let coords: Vec<(i32, i32)> = self.sinks.iter().map(|s| (s.x, s.y)).collect();
        // group label and sink indexing pair. group label can be used later with branchs to fully
        // specify symmetry clock tree topology
        // As a result, the grp label is in increased-order
        // (u32,usize) = (grp_label,sink_index)
        let grp2id: Vec<(u32, usize)> = group(&coords, &branchs);

        // STEP 3: bottom-up merge

        // reverse branchs to bottom-up order
        // hashmap to store map between merge level and target wirelength
        branchs.reverse();

        // to store child node generated in each level
        let mut nodes: Vec<Rc<TreeNode>> = Vec::new();

        // prepare most bottom node in the tree(Sinks)
        for (_, d) in &grp2id {
            nodes.push(Rc::new(TreeNode::new_sink_node(*d)));
        }

        let mut childs: Vec<(i32, i32)> = grp2id
            .iter()
            .map(|d| {
                let s = &self.sinks[d.1];
                (s.x, s.y)
            })
            .collect();

        let mut level2length: HashMap<u8, u32> = HashMap::new();
        // store roots location of nodes in same level when merging
        let mut new_childs: Vec<(i32, i32)> = Vec::new();
        // store roots TreeNode of nodes in same level when merging
        let mut new_child_nodes: Vec<Rc<TreeNode>> = Vec::new();
        println!("{:?}", branchs);
        for (i, b) in branchs.iter().enumerate() {
            let level = max_level - i as u8;
            let mut target_len = u32::MIN;
            let mut one_merge_childs: Vec<(i32, i32)> = Vec::new();
            let mut child_nodes: Vec<Rc<TreeNode>> = Vec::new();
            for (j, s) in childs.iter().enumerate() {
                let n = j as u32;
                one_merge_childs.push(*s);
                child_nodes.push(nodes[j].clone());
                // if n % b == 0 && n != 0, it means one_merge_childs is not empty and same group childs are all collected in the one_merge_childs.
                // one_merge_childs is ready to be load into one_merge_inst
                if (n + 1) % *b == 0 {
                    // (1) update MergeUnit
                    let mut one_merge_inst = MergeUnit::new();
                    one_merge_inst.load_sink(&one_merge_childs);
                    one_merge_inst.set_level(level);
                    // get target length in current tree level by comparing bewteen same level of MergeUnit
                    if one_merge_inst.common_length() > target_len {
                        target_len = one_merge_inst.common_length();
                    }

                    // update roots, which is used in next iteration as childs
                    new_childs.push(one_merge_inst.root.clone());

                    self.merge.push(one_merge_inst);

                    // (2) update TreeNode
                    // current MergeUnit index
                    let current_merge_idx = self.merge.len();
                    let mut a_node = TreeNode::new_dot_node(current_merge_idx);
                    a_node.level = level;
                    let node = Rc::new(a_node);
                    *node.child.borrow_mut() = child_nodes.clone();
                    // update root nodes, which is used in next iteration as child nodes
                    new_child_nodes.push(node);

                    // reset next iter childs
                    one_merge_childs.clear();
                    // reset child nodes
                    child_nodes.clear();
                }
            }
            level2length.insert(level, target_len);
            // childs.clear();
            // nodes.clear();

            // iterate MergeUnit and TreeNode
            childs = new_childs.clone();
            nodes = new_child_nodes.clone();
            new_childs.clear();
            new_child_nodes.clear();
        }
        println!("MergeUnit {}", self.merge.len());
        println!("{:?}", level2length);
        for m in self.merge.iter_mut() {
            // let level = m.level;
            // let target_len = *level2length.get(&level).unwrap();
            // let margin = target_len - m.common_length();
            // if margin > 200 {
            //     m.adjust_root(margin);
            // }
            m.merge();
        }
        let total_estimate_wire = self.merge.iter().fold(0, |acc, x| acc + x.length());
        println!(
            "pre-merge finished, estimated wirelength:{}",
            total_estimate_wire
        );
        if nodes.len() == 1 {
            let root = nodes[0].clone();
            self.root = Some(RefCell::new(root));
            println!("Successfully create tree root");
        } else {
            println!("Node failed, there {} nodes", nodes.len());
        }
        level2length
    }

    // buffering select buffer size combination (insertion solution) into candidate insertion points on un-buffered symmetric clock tree.
    // This function regards level2length as the topology structure when optimization, for simplity
    fn buffering(&mut self, _topo: &HashMap<u8, u32>, _buffers: &HashMap<String, Value>) {
        let mut buf_count = 0;
        if let Some(root) = &self.root {
            // insert max buffer at root
            let root_idx: usize = root.borrow().get_dot_node_data().unwrap();
            let root_location = self.merge[root_idx - 1].root;
            let buffer = Buffer {
                buffer_name: format!("{}_{}", "BUFH_X16M_A9TL40", buf_count),
                model_name: String::from("BUFH_X16M_A9TL40"),
                location: root_location,
            };
            self.buffers.push(buffer);
            buf_count += 1;
        }
        println!("Total {} buffer inserted", buf_count);
    }
}

#[derive(Default, Debug)]
pub struct Sink {
    name: String, // cell name
    x: i32,
    y: i32,
    load_cap: f32,
}

pub struct Buffer {
    pub buffer_name: String,
    pub model_name: String,
    pub location: (i32, i32),
    // in_pin: BufInPin,
    // out_pin: BufOutPin,
}

// pub struct BufInPin {
//     name: String,
//     location: (i32, i32),
//     cap: f32,
// }

// pub struct BufOutPin {
//     name: String,
//     location: (i32, i32),
// }

// fn buffering() -> HashMap<()> {}

// fn get_center(coords: &[(i32, i32)]) -> (i32, i32) {
//     let mut top_most = (0,0);
//     let mut bott_most = (0,0);
//     let mut left_most = (0,0);
//     let mut right_most = (0,0);
//     let (mut min_x,mut max_x,mut min_y,mut max_y) = (i32::MAX,i32::MIN,i32::MAX,i32::MIN);
//     for d in coords {
//         if d.0 < min_x {
//             min_x = d.0;
//             left_most = *d;
//         }
//         if d.0 > max_x {
//             max_x = d.0;
//             right_most = *d;
//         }
//         if d.1 < min_y {
//             min_y = d.1;
//             bott_most = *d;
//         }
//         if d.1 > max_y {
//             max_y = d.1;
//             top_most = *d;
//         }
//     }
//     (((top_most.0 + bott_most.0)/2) as i32, ((left_most.1 + right_most.1)/2) as i32)
// }

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

fn get_cost_value(coords: &[(i32, i32)]) -> u32 {
    let mut top_most = (0, 0);
    let mut bott_most = (0, 0);
    let mut left_most = (0, 0);
    let mut right_most = (0, 0);
    let (mut min_x, mut max_x, mut min_y, mut max_y) = (i32::MAX, i32::MIN, i32::MAX, i32::MIN);
    for d in coords {
        if d.0 < min_x {
            min_x = d.0;
            left_most = *d;
        }
        if d.0 > max_x {
            max_x = d.0;
            right_most = *d;
        }
        if d.1 < min_y {
            min_y = d.1;
            bott_most = *d;
        }
        if d.1 > max_y {
            max_y = d.1;
            top_most = *d;
        }
    }
    (top_most.1 - bott_most.1).abs() as u32 + (left_most.0 - right_most.0).abs() as u32
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

// fn buffering(topology: &mut [TreeNode]) -> MyRes<()> {
//     Ok(())
// }
