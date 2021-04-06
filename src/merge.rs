// try to implement manhattan style merge that given set of point, get the
// parent node which promise least total wire source and equal manhattan distance
// to each child node
// Note: there is no limit on number of given points, that is , it support 
// non-binary tree merge
#[derive(Default)]
struct MergeUnit {
    sink: Vec<(i32,i32)>,
    pub root: (i32,i32),
    x_range:(i32,i32),
    y_range:(i32,i32),
    path: Vec<Path>,
    total_length: u32,
    pub estimate_length: u32,
    level : u8,
}

struct Path {
    from:(i32,i32),
    to:(i32,i32),
}
impl Path {
    pub fn if_veritcal(&self) -> bool {
        self.from.0 == self.to.0
    } 
    pub fn if_horizontal(&self) -> bool {
        self.from.1 == self.to.1
    }
    pub fn if_turn(&self) -> bool {
        self.from.0 != self.to.0 && self.from.1 != self.to.1
    }
    pub fn if_invalid(&self) -> bool {
        self.from.0 == self.to.0 && self.from.1 == self.to.1
    }
}

impl MergeUnit {
    pub fn new() -> Self {
        Default::default()
    }
    pub fn set_level(&self,level:u8) {
        self.level = level;
    }
    pub fn adjust_root(&mut self,offset:(i32,i32)) {
        self.root = (self.root.0 + offset.0, self.root.1 + offset.1); 
    }
    pub fn load_sink(&mut self,sink:&[(i32,i32)]) {
        let (mut x_min,mut x_max,mut y_min,mut y_max) = i32::MAX,i32::MIN,i32::MAX,i32::MIN;
        for d in sink {
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
            self.sink.push(d);
        }
        self.x_range = (x_min,x_max);
        self.y_range = (y_min,y_max);
    }
    pub fn analyze(&self) {
        self.root = ((x_range.0 + x_range.1)/2,(y_range.0 + y_range.1)/2);
        let mut estimate_length = (((self.x_range.0-self.x_range.1).abs()+(self.y_range.0-self.y_range.1).abs())/2) as u32;
        for d in &self.sink {
            estimate_length += (((d.0-self.root.0).abs()+(d.1-self.root.1).abs())/2) as u32;
        }
        self.estimate_length = estimate_length;
    }
    pub fn merge(&mut self) {
        // first analysis
        
        //
    }
}


