use core::cell::RefCell;
use std::rc::Rc;
use std::rc::Weak;

pub enum NodeData {
    Sink(usize),
    Buffer(usize),
    Dot(usize),
}


// reference https://joshondesign.com/2020/04/08/rust5_tree
pub struct TreeNode {
    pub child: RefCell<Vec<Rc<TreeNode>>>,
    pub parent: RefCell<Weak<TreeNode>>,
    // Three types can be hold in node_data: Clock buffer(ViComponent), Sink(ViComponent),Tappoint(coord)
    node_data: NodeData,
    pub level:u8, // node level
}

impl TreeNode {
	// create a sink node
	pub fn new_sink_node(idx:usize) -> Self {
		TreeNode {
			node_data:NodeData::Sink(idx),
			child: RefCell::new(Vec::new()),
			parent: RefCell::new(Default::default()),
			level: 0,
		}
	}
	// create a tap point node
	pub fn new_dot_node(idx:usize) -> Self {
		TreeNode {
			node_data:NodeData::Dot(idx),
			child: RefCell::new(Vec::new()),
			parent: RefCell::new(Default::default()),
			level: 0,
		}
	}
	pub fn get_dot_node_data(&self) -> Option<usize> {
		if let NodeData::Dot(u) = self.node_data {
			Some(u)
		} else {
			None
		}
	}


	pub fn insert_buffer(&mut self, idx:usize) {
		self.node_data = NodeData::Buffer(idx);
	}
	pub fn label(&self) -> String {
		match self.node_data {
			NodeData::Sink(i) => format!("n_s{}", i),
			NodeData::Buffer(i) => format!("n_b{}", i),
			NodeData::Dot(i) => format!("n_d{}",i),
		}
	}
	pub fn if_leaf_node(&self) -> bool {
		if let NodeData::Sink(_) = self.node_data {
			true
		} else {
			false
		}
	}
	// pub fn add_child(&mut self,childs:&Vec<TreeNode>) {
	// 	let 
	// }

}



