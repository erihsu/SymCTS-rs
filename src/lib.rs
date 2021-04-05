use cts_plugin::*;
use std::collections::HashSet;
use std::cell::RefCell;
use std::fs;
use std::path::Path;
use std::rc::Rc;
use std::rc::Weak;
use libloading::Library;
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
}


impl ClockTree {
    fn new(name:&str) -> Self {
        ClockTree {
            name:name.to_string(),
            ..Default::default()
        }
    }

    fn prepare_from_def(&mut self,file:P)
    where 
        P: AsRef<Path>,
{
    let cts_lib = Library::new("").expect("load library");
    let mut design_plugin : libloading::Symbol<extern "Rust" fn () -> Box<dyn DesignPlugin> > = unsafe {lib.get(b"new_design_plugin")}.expect("load symbol");
    let buf_data = file.read_to_string()?;
    let def_design = design_plugin.import_def(&buf_data)?;
    let sinks:Vec<(String, (i32, i32), i8)> = def_design.get_clock_sinks(&self.name);
    let mut sink_type = HashSet::new();
    sinks.iter().for_each(|x|sink_type.insert(x.0.to_string()));
    let pdk_lib = Library::new("").expect("load library");
    let mut pdk_plugin : libloading::Symbol<extern "Rust" fn() -> Box<dyn DesignPlugin> > = unsafe {lib.get(b"new_pdk_plugin")}.expect("load symbol");
    pdk_plugin.login(); // read env and login
    let mut sink_offset: HashMap<String,(f32,f32)> = HashMap::new();
    let mut sink_cap:HashMap<String,f32> = HashMap::new();
    let unit_glue = LEF2DBU::new(1000,1000); // mannually only right now
    for d in sink_type {
        let offset = pdk_plugin.get_sink_clk_pin_offset(d)?;
        let cap = pdk_plugin.get_sink_cap(d)?;
        sink_offset.insert(d.to_string(),offset);
        sink_cap.insert(d.to_string(),cap);
    }
    let (mut x_min,mut x_max,mut y_min,mut y_max) = (i32::MAX,i32::MIN,i32::MAX,i32::MIN);
    self.sinks = sinks.iter().map(|x|{
        let orient = x.2;
        let (origin_x,origin_y) = x.1;
        let (offset_x,offset_y) = sink_offset.get(x.0);
        let offset_dbu = (unit_glue.to_dbu(offset_x),unit_glue.to_dbu(offset_y));
        let adjust_offset_dbu = adjust_pin_location(offset_dbu.0,offset_dbu.1,orient);
        let load_cap = *sink_cap.get(x.0).unwrap();
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
            name:x.to_string(),
            x:adjust_offset_dbu.0,
            y:adjust_offset_dbu.1,
            load_cap,
        }
    }).collect();
    self.x_range = (x_min,x_max);
    self.y_range = (y_min,y_max);
}
    fn gen_topology(&mut self) {
        // step 1: bnp and insert pseudo sink
        let branchs = vec![];
        let n = self.sinks.len();
        while n > 1 {
            for i in 2..=n {
                if n%i == 0 {
                    n /= i;
                    branchs.push(i);
                    break
                }
            }
        }
        // add pseudo sinks with zero capload
        let target_num = branchs.iter().fold(0,|acc,x|acc*x);
        let pseudo_sink = target_num - n;
        let mut rng = rand::thread_rng();
        for _ in 0..pseudo_sink {
            self.sinks.push(Sink{
                x:rng.gen(self.x_range.0,self.x_range.1),
                y:rng.gen(self.y_range.0,self.y_range.1),
                ..Default::default()
            })
        }
        println!("expect {} sinks to get symmetrical structure, insert {} pseudo sink into {} real sink",target_num,pseudo_sink,n);
        // step 2 : top-down parition
        let coords = self.sinks.iter().map(|s|(s.x,s.y)).collect();

        // sink indexing and group label pair. group label can be used later with branchs to fully
        // specify symmetry clock tree topology
        let grp2id :Vec<(i32,usize)> = group(&coords,&branchs);
        
        // step 3: bottom-up merge

    }
}

#[derive(Default)]
pub struct Sink {
    name : String, // cell name
    x: i32,
    y: i32,
    load_cap: f32,
}

// reference https://joshondesign.com/2020/04/08/rust5_tree
pub struct TreeNode {
    pub child: RefCell<Vec<Rc<TreeNode>>>,
    parent: Option<Rc<Weak<TreeNode>>>,
    // Three types can be hold in node_data: Clock buffer(ViComponent), Sink(ViComponent),Tappoint(VdbDot)
    node_data: NodeData,

    // Sym-CTS related
    radius: f32, // max length in the region
    fanout: i32,
    node_cap: f32, // total load cap
    wires: Vec<Wire>,
}

pub enum NodeData {
    Sink(Box<Sink>),
    Buffer(Box<Buffer>),
    Dot(VdbDot),
}

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
    let mut n:u32 = 0;
    coords.iter().for_each(|x|{n+=1;sum_x += x.0;sum_y += x.1});
    let center_x = sum_x/n as i32;
    let center_y = sum_y/n as i32;
    (center_x,center_y)
}

// to simplify get max distance between two farest point, just use perimeter to represent max
// distance
fn get_max_distance(coords:&[(i32,i32)]) -> u32 {
    let mut dis = 0;
    let len = coords.len() as i32;
    for i in 0..len-1 {
        dis += (coords[i].0 - coords[i].1).abs() + (coords[i+1].1 - coords[i+1].1).abs();
    }
    dis += (coords[len-1].0 - coords[0].0).abs() + (coords[len-1].1 -coords[0].1).abs();
    dis
}

fn group(coords:&[(i32,i32)],branchs:&Vec<i32>) -> Vec<(u32,usize)> {
    let result = Vec::new();
    for (i,b) in branchs {
        let mut grps = Vec::new();
        if i == 0 {
            let idxs = 0..len(coords);
            grps = find_group(coords,(0,idxs),b);
        } else if i < branchs.len() - 1{
            let mut new_d = Vec::new();
            for d in grps {
                let next_grps = find_group(coords,d,b);
                new_d.extend(&next_grps);
            }
            grps = new_d;
        } else {
            for d in grps {
                let next_grps = find_group(coord,d,b);
                for v in next_grps {
                    // Vec<(u32,Vec<usize>)> become Vec<(u32,vec![usize])>
                    result.push((v.0,v.1[0]));
                }
            }
        }
    }
    result
}


use std::f32::consts::PI;
fn find_group(coords:&[(i32,i32)],grp2id:(u32,Vec<usize>),grpn_next:u32) -> Vec<(u32,Vec<usize>)> {
    let grp_id_next = vec![];
    let start_gid_next = (grpn_next-1)*grp2id.0;
    for i in 0..grpn_next {
        grp_id_next.push(start_gid_next + i);
    }
    let locations: Vec<(i32,i32)> = grp2id.1.iter().map(|x|(x,coords[x])).collect();
    let center = get_center(&locations);
    let relative_location_phase:Vec<f32> = locations.iter().map(|d|{
        let x = (d.0-center.0) as f32;
        let y = (d.1-center.1) as f32;
        y.atan2(x)
    }.collect();
    let mut pre_sort_data = grp2id.1.iter().zip(&relative_location_phase.iter()).collect();
    pre_sort_data.sort_by(|a,b|a.1.cmp(&b.1));
    let sorted_idx:Vec<usize> = pre_sort_data.iter().map(|x|x.0).collect();
    let cut_step = sorted_idx.len()/grpn_next as i32;
    let mut min_dis = u32::MAX;
    let mut min_dis_cut = 0;
    for start_cut in 0..cut_step {
        let mut max_dis = 0;
        for i in 0..grpn_next {
            let idxs =  if i == grpn_next - 1{
                let mut idxs = sorted_idx[i*cut_step + start_cut:-1];
                idxs.extend(sorted_idx[0:cut_step]);
                idxs    
            } else {
                sorted_idx[i*cut_step + start_cut : (i+1)*cut_step + start_cut] 
            }
            let locs = idxs.iter().map(|x:usize|coords[x]).collect();
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
        let idxs = if i == grpn_next - 1{
            let mut idxs = sorted_idx[i*cut_step + min_dis_cut:-1];
            idxs.extend(sorted_idx[0:cut_step]);
            idxs
        } else {
            sorted_idx[i*cut_step + start_cut : (i+1)*cut_step + start_cut] 
        }
        result.push((gid,idxs))
    }
    result
}

fn buffering(topology: &mut [TreeNode]) -> MyRes<()> {
    Ok(())
}
