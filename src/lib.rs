#![allow(dead_code)]
use cts_plugin::{DesignPlugin,PdkPlugin,CTSPluginRes};
use std::collections::{HashSet,HashMap};
use std::cell::RefCell;

use std::path::Path;
use std::fs;
use std::rc::Rc;
use std::rc::Weak;
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
    name:String,
    x_range:(i32,i32),
    y_range:(i32,i32),
    sinks: Vec<Sink>,
    buffers: Vec<Buffer>,
    merge: Vec<MergeUnit>,
}


impl ClockTree {
    fn new(name:&str) -> Self {
        ClockTree {
            name:name.to_string(),
            ..Default::default()
        }
    }

    fn prepare_from_def<P>(&mut self,file:P) -> CTSPluginRes<()>
    where 
        P: AsRef<Path>,
{
    let cts_lib = Library::new("").expect("load library");
    let new_design_plugin : libloading::Symbol<extern "Rust" fn () -> Box<dyn DesignPlugin> > = unsafe {cts_lib.get(b"new_design_plugin")}.expect("load symbol");
    let adjust_pin_location : libloading::Symbol<extern "Rust" fn (i32,i32,i8) -> (i32,i32)> = unsafe {cts_lib.get(b"adjust_pin_location")}.expect("load symbol");
    let mut my_design = new_design_plugin();
    let buf_data = fs::read_to_string(file)?;
    my_design.import_def(&buf_data)?;
    let sinks:Vec<(String, (i32, i32), i8)> = my_design.get_clock_sinks(&self.name)?;
    let mut sink_type = HashSet::new();
    sinks.iter().for_each(|x|{sink_type.insert(x.0.to_string());});
    let pdk_lib = Library::new("").expect("load library");
    let new_pdk_plugin : libloading::Symbol<extern "Rust" fn() -> Box<dyn PdkPlugin> > = unsafe {pdk_lib.get(b"new_pdk_plugin")}.expect("load symbol");
    let my_pdk = new_pdk_plugin(); // read env and login
    let mut sink_offset: HashMap<String,(f32,f32)> = HashMap::new();
    let mut sink_cap:HashMap<String,f32> = HashMap::new();
    let dbu_factor = my_design.get_length_dbu()?;
    let dbu_factor = dbu_factor as f32;
    for d in sink_type {
        let offset = my_pdk.get_sink_clk_pin_offset(&d)?;
        let cap = my_pdk.get_sink_cap(&d)?;
        sink_offset.insert(d.to_string(),offset);
        sink_cap.insert(d.to_string(),cap);
    }
    let (mut x_min,mut x_max,mut y_min,mut y_max) = (i32::MAX,i32::MIN,i32::MAX,i32::MIN);
    self.sinks = sinks.iter().map(|x|{
        let orient = x.2;
        let (origin_x,origin_y) = x.1;
        let (offset_x,offset_y) = *sink_offset.get(&x.0).unwrap();
        let offset_dbu = ((dbu_factor*offset_x) as i32,(dbu_factor*offset_y) as i32);
        let adjust_offset_dbu = adjust_pin_location(offset_dbu.0,offset_dbu.1,orient);
        let load_cap = *sink_cap.get(&x.0).unwrap();
        if adjust_offset_dbu.0 < x_min {
            x_min = adjust_offset_dbu.0;
        }
        if adjust_offset_dbu.0 > x_max {
            x_max = adjust_offset_dbu.0;
        }
        if adjust_offset_dbu.1 < y_min {
            y_min = adjust_offset_dbu.1
        }
        if adjust_offset_dbu.1 > y_max {
            y_max = adjust_offset_dbu.1
        }
        Sink{
            name:x.0.to_string(),
            x:adjust_offset_dbu.0,
            y:adjust_offset_dbu.1,
            load_cap,
        }
    }).collect();
    self.x_range = (x_min,x_max);
    self.y_range = (y_min,y_max);
    Ok(())
}
    fn gen_topology(&mut self) {
        // step 1: bnp and insert pseudo sink
        let mut branchs = vec![];
        let mut n = self.sinks.len() as u32;
        while n > 1 {
            for i in 2..=n {
                if n%i == 0 {
                    n /= i;
                    branchs.push(i as u32);
                    break
                }
            }
        }
        println!("branch number planning finished,result:{:?}",branchs);
        // add pseudo sinks with zero capload
        let target_num = branchs.iter().fold(0,|acc,x|acc*x);
        let pseudo_sink = target_num - n;
        let mut rng = rand::thread_rng();
        for _ in 0..pseudo_sink {
            self.sinks.push(Sink{
                x:rng.gen_range(self.x_range.0..self.x_range.1),
                y:rng.gen_range(self.y_range.0..self.y_range.1),
                ..Default::default()
            })
        }
        println!("expect {} sinks to get symmetrical structure, insert {} pseudo sink into {} real sink",target_num,pseudo_sink,n);
        // step 2 : top-down parition
        let coords:Vec<(i32,i32)> = self.sinks.iter().map(|s|(s.x,s.y)).collect();

        // sink indexing and group label pair. group label can be used later with branchs to fully
        // specify symmetry clock tree topology
        // As a result, the grp label is in increased-order
        let grp2id :Vec<(u32,usize)> = group(&coords,&branchs);
        
        // step 3: bottom-up merge
        let rev_branch = branchs.reverse();
        let childs:Vec<(i32,i32)> = grp2id.iter().map(|d|{let sink self.sink[d];(sink.x,sink.y)}).collect();
        let mut new_childs = Vec::new();
        for (level,b) in rev_branch.iter().enumerate() {
            for (i,s) in childs.iter().enumerate() {
                let mut one_merge_childs = Vec::new();
                if i%b != 0 {
                    one_merge_childs.push(s);
                } else {
                    let mut one_merge_inst = MergeUnit::new();
                    one_merge_inst.load_sink(one_merge_childs);
                    one_merge_inst.analyze();
                    one_merge_inst.set_level(level as u8);
                    self.merge.push(one_merge_inst);
                    new_childs.push(one_merge_inst.root);
                    one_merge_childs.clear();

                }
            }
            childs.clear();
            childs = new_childs;
            new_childs = clear();
        }
        let total_estimate_wire = self.merge.iter().fold(0,|acc,x|acc + x.estimate_length);
        println!("pre-merge finished, estimated wirelength:{}",total_estimate_wire);
    }
fn buffering(&mut self) {

}
}

#[derive(Default)]
pub struct Sink {
    name : String, // cell name
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
    location:(i32,i32),
    cap:f32,
}

pub struct BufOutPin {
    name :String,
    location: (i32,i32),

}

// get geometry center
fn get_center(coords:&[(i32,i32)]) -> (i32,i32) {
    let mut  sum_x:i64 = 0;
    let mut sum_y:i64 = 0;
    let mut n:i64 = 0;
    coords.iter().for_each(|x|{n+=1;sum_x += x.0 as i64;sum_y += x.1 as i64});
    let center_x = (sum_x/n) as i32;
    let center_y = (sum_y/n) as i32;
    (center_x,center_y)
}

// to simplify get max distance between two farest point, just use perimeter to represent max
// distance
fn get_max_distance(coords:&[(i32,i32)]) -> u32 {
    let mut dis = 0;
    let len = coords.len();
    for i in 0..len-1 {
        dis += (coords[i].0 - coords[i].1).abs() + (coords[i+1].1 - coords[i+1].1).abs();
    }
    dis += (coords[len-1].0 - coords[0].0).abs() + (coords[len-1].1 -coords[0].1).abs();
    dis as u32
}

fn group(coords:&[(i32,i32)],branchs:&Vec<u32>) -> Vec<(u32,usize)> {
    let mut result = Vec::new();
    for (i,b) in branchs.iter().enumerate() {
        let mut grps = Vec::new();
        if i == 0 {
            let idxs = (0..coords.len()).collect();
            grps = find_group(coords,(0,idxs),*b);
        } else if i < branchs.len() - 1{
            let mut new_d = Vec::new();
            for d in grps {
                let next_grps = find_group(coords,d,*b);
                for g in next_grps {
                    new_d.push(g);
                }
            }
            grps = new_d;
        } else {
            for d in grps {
                let next_grps = find_group(coords,d,*b);
                for v in next_grps {
                    // Vec<(u32,Vec<usize>)> become Vec<(u32,vec![usize])>
                    result.push((v.0,v.1[0]));
                }
            }
        }
    }
    result
}



fn find_group(coords:&[(i32,i32)],grp2id:(u32,Vec<usize>),grpn_next:u32) -> Vec<(u32,Vec<usize>)> {
    let mut grp_id_next = vec![];
    let start_gid_next = (grpn_next-1)*grp2id.0;
    for i in 0..grpn_next {
        grp_id_next.push(start_gid_next + i);
    }
    let locations: Vec<(i32,i32)> = grp2id.1.iter().map(|x|coords[*x]).collect();
    let center = get_center(&locations);
    let relative_location_phase:Vec<i32> = locations.iter().map(|d|{
        let x = (d.0-center.0) as f32;
        let y = (d.1-center.1) as f32;
        (y.atan2(x)*10000.0) as i32
    }).collect();
    let mut pre_sort_data:Vec<(&usize,&i32)> = grp2id.1.iter().zip(&mut relative_location_phase.iter()).collect();
    pre_sort_data.sort_by(|a,b|a.1.cmp(&b.1));
    let sorted_idx:Vec<usize> = pre_sort_data.iter().map(|x|x.0.clone()).collect();
    let cut_step = sorted_idx.len() as u32/grpn_next;
    let mut min_dis = u32::MAX;
    let mut min_dis_cut = 0;
    // find min_dis_cut that achieve minimal parition cost
    for start_cut in 0..cut_step {
        let mut max_dis = 0;
        for i in 0..grpn_next {
            let mut idxs:Vec<usize> = Vec::new();
            if i == grpn_next - 1 {
                let start:usize = (i*cut_step + start_cut) as usize;
                idxs.extend_from_slice(&sorted_idx[start..]);
                let end:usize = cut_step as usize;
                idxs.extend_from_slice(&sorted_idx[..end]);

            } else {
                let start = (i*cut_step + start_cut) as usize;
                let end = ((i+1)*cut_step + start_cut) as usize;
                idxs.extend_from_slice(&sorted_idx[ start.. end]);
            };
            let locs:Vec<(i32,i32)> = idxs.iter().map(|x|coords[*x]).collect();
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
    for (i,gid) in grp_id_next.iter().enumerate(){
        let i = i as u32;
        let mut idxs:Vec<usize> = Vec::new();
        if i == grpn_next - 1{
            let start = (i*cut_step + min_dis_cut) as usize;
            idxs.extend_from_slice(&sorted_idx[start..]);
            let end = (cut_step) as usize;
            idxs.extend_from_slice(&sorted_idx[0..end]);
        } else {
            let start = (i*cut_step + min_dis_cut) as usize;
            let end = ((i+1)*cut_step + min_dis_cut) as usize;
            idxs.extend_from_slice(&sorted_idx[ start.. end]);
             
        };
        result.push((*gid,idxs))
    }
    result
}

// fn buffering(topology: &mut [TreeNode]) -> MyRes<()> {
//     Ok(())
// }
