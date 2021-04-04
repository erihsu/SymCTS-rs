use cts_plugin::*;

use std::cell::RefCell;

use std::path::Path;
use std::rc::Rc;
use std::rc::Weak;

enum MyError {
    IOErr,
    BNPErr,
    DivisionErr,
}

type MyRes<T> = Result<T, MyError>;



pub struct ClockTree {
    sinks: Vec<Sink>,
    buffers: Vec<Buffer>,
}


impl ClockTree {
    fn prepare_from_pdk(&mut self,file:P)
    where 
        P: AsRef<Path>,
{

}

}


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

fn get_sinks_from_def<P>(file: P) -> MyRes<Vec<Sink>>
where
    P: AsRef<Path>,
{
    // parse using def-parser

    //
}

fn gen_topology(sinks: &[Sink]) -> MyRes<Vec<TreeNode>> {}

fn buffering(topology: &mut [TreeNode]) -> MyRes<()> {
    Ok(())
}
