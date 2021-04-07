// try to implement manhattan style merge that given set of point, get the
// parent node which promise least total wire source and equal manhattan distance
// to each child node
// Note: there is no limit on number of given points, that is , it support 
// non-binary tree merge

// Merge Unit 
//  --------------------
//           | 
//           |
//  --------------------
//           |
//           |
//  --------------------
#[derive(Default)]
pub struct MergeUnit {
    sink: Vec<(i32,i32)>,
    pub root: (i32,i32),
    x_range:(i32,i32),
    y_range:(i32,i32),
    path: Vec<Path>,
    if_veritcal:bool,
    pub level : u8,
}


// path can be horizontal, vertical, turning.
// if turning , always veritcal followed by horizontal
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
    pub fn length(&self) -> i32 {
        (self.from.0 - self.to.0).abs() + (self.from.1 - self.to.1).abs()
    }
}

impl MergeUnit {
    pub fn new() -> Self {
        Default::default()
    }
    pub fn length(&self) -> i32 {
        self.path.iter().fold(0,|acc,x|acc + x.length())
    }
    pub fn set_level(&mut self,level:u8) {
        self.level = level;
    }
    pub fn adjust_root(&mut self,offset:(i32,i32)) {
        self.root = (self.root.0 + offset.0, self.root.1 + offset.1); 
    }
    pub fn range_length(&self) -> u32 {
        (self.x_range.1 -self.x_range.0 + self.y_range.1 - self.y_range.0) as u32
    }
    // in this function , target length is equal to len(x_range) + (y_range).len() after adjustment
    pub fn adjust_range(&mut self,target_length:u32) {
        let delta = ((target_length - self.range_length())/4) as i32;
        if delta > 0{
           self.x_range = (self.x_range.0 -delta,self.x_range.1 + delta);
           self.y_range = (self.y_range.0 - delta, self.y_range.1 + delta);
        }
        
    }
    pub fn load_sink(&mut self,sink:&[(i32,i32)]) {
        let (mut x_min,mut x_max,mut y_min,mut y_max) = (i32::MAX,i32::MIN,i32::MAX,i32::MIN);
        sink.iter().for_each(|d|{
            if d.0 < x_min {
                x_min = d.0;
            }
            if d.0 > x_max {
                x_max = d.0;
            }
            // most bottom
            if d.1 < y_min {
                y_min = d.1;
            }
            // most top
            if d.1 > y_max {
                y_max = d.1;
            }
            self.sink.push(*d);
        });
        self.x_range = (x_min,x_max);
        self.y_range = (y_min,y_max);

        // analyze
        let range_x_len = x_max - x_min;
        let range_y_len = y_max - y_min;
        if range_x_len > range_y_len {
            self.if_veritcal = true;
            let delta = range_x_len - range_y_len;
            let y_extend = (delta / 2) as i32;
            self.y_range = (self.y_range.0 - y_extend, self.y_range.1 + y_extend);
        } else {
            self.if_veritcal = false;
            let delta = range_y_len - range_x_len;
            let x_extend = (delta / 2) as i32;
            self.x_range = (self.x_range.0 - x_extend, self.x_range.1 + x_extend);
        }
        self.root = (((self.x_range.0 + self.x_range.1)/2) as i32, ((self.x_range.0 + self.x_range.1)/2 ) as i32);
    }

    pub fn merge(&mut self) {
        // shape:
        //     --------------
        //           |
        //           |
        //     ------------- 
        if self.if_veritcal {
            let mut top_left_offset = i32::MAX;
            let mut top_right_offset = i32::MIN;
            let mut bottom_left_offset = i32::MAX;
            let mut bottom_right_offset = i32::MIN;
            for x in &self.sink {
                                //                              -------
                // tap point is only exist on      |
                //                              -------
                let mut tap_point = self.root; 
                // first analysis
                let (x_offset,y_offset) = (x.0-self.root.0,x.1-self.root.1);
                // case 1 : at the top-right region of merge unit
                if x_offset > 0 && y_offset > 0 {
                    let mismatch = (self.x_range.1 - x.0) - (self.y_range.1 - x.1);
                    // longer common path 
                    if mismatch > 0 {
                            tap_point.1 = self.y_range.1;
                            tap_point.0 = x.0 + (mismatch/2) as i32;
                            if tap_point.0 > top_right_offset {
                                top_right_offset = tap_point.0;
                            }
                            // add path from tap point to sink, because vertical first
                            self.path.push(Path{from:tap_point,to:*x});
                    }
                    // shorter common path
                    else {
                        tap_point.0 = self.root.0;
                        tap_point.1 = self.y_range.1 + (mismatch/2) as i32;
                        // add path from sink to tap point, because vertical first
                        self.path.push(Path{from:*x,to:tap_point});
                    }
                }  
                // case 2: at the bottom-right region of merge unit
                else if x_offset > 0 && y_offset < 0 {
                    let mismatch = (self.x_range.1 - x.0) - (self.y_range.1 - x.1);
                    // longer common path
                    if mismatch > 0 {
                        tap_point.1 = self.y_range.0;
                        tap_point.0 = x.0 + (mismatch/2) as i32;
                        if tap_point.0 > bottom_right_offset {
                            bottom_right_offset = tap_point.0;
                        }
                            // add path from tap point to sink, because vertical first
                            self.path.push(Path{from:tap_point,to:*x});
                    } 
                    // shorter common path
                    else {
                        tap_point.0 = self.root.0;
                        tap_point.1 = self.y_range.0 - (mismatch/2) as i32;
                        // add path from sink to tap point, because vertical first
                        self.path.push(Path{from:*x,to:tap_point});
                    }
                }
                // case 3: at the top-left region of merge unit
                else if x_offset < 0 && y_offset > 0 {
                    let mismatch = (x.0 - self.x_range.0) - (self.y_range.1 - x.1);
                    // longer common path
                    if mismatch > 0 {
                        tap_point.1 = self.y_range.1;
                        tap_point.0 = x.0 - (mismatch/2) as i32;
                        if tap_point.0 < top_left_offset {
                            top_left_offset = tap_point.0;
                        }
                            // add path from tap point to sink, because vertical first
                            self.path.push(Path{from:tap_point,to:*x});
                    }
                    // shorter common path 
                    else {
                        tap_point.0 = self.root.0;
                        tap_point.1 = self.y_range.1 + (mismatch/2) as i32;
                        // add path from sink to tap point, because vertical first
                        self.path.push(Path{from:*x,to:tap_point});
                    }
                }
                // case 4: at the bottom-left region of merge unit
                else if x_offset < 0 && y_offset < 0 {
                    let mismatch = (x.0-self.x_range.0) - (x.1-self.y_range.0);
                    // longer common path
                    if mismatch > 0 {
                        tap_point.1 = self.y_range.0;
                        tap_point.0 = x.0 - (mismatch)/2 as i32;
                        if tap_point.0 < bottom_left_offset {
                            bottom_left_offset = tap_point.0;
                        }
                            // add path from tap point to sink, because vertical first
                            self.path.push(Path{from:tap_point,to:*x});
                    }
                    // shorter common path
                    else {

                        tap_point.0 = self.root.0;
                        tap_point.1 = self.y_range.0 - (mismatch/2) as i32;
                        // add path from sink to tap point, because vertical first
                        self.path.push(Path{from:*x,to:tap_point});
                    }
                }
            }
        self.path.push(Path{from:(top_left_offset,self.y_range.1),to:(top_right_offset,self.y_range.1)});
        self.path.push(Path{from:(bottom_left_offset,self.y_range.0),to:(bottom_right_offset,self.y_range.1)});
        self.path.push(Path{from:(self.root.0,self.y_range.1),to:(self.root.0,self.y_range.0)});
        } 
        // shape:
        //    |                |
        //    |                |
        //    | -------------- |
        //    |                |
        //    |                |
        else {
            let mut top_left_offset = i32::MAX;
            let mut top_right_offset = i32::MIN;
            let mut bottom_left_offset = i32::MAX;
            let mut bottom_right_offset = i32::MIN;
            for x in &self.sink {
                let mut tap_point = self.root; 
                // first analysis
                let (x_offset,y_offset) = (x.0-self.root.0,x.1-self.root.1);
                // case 1 : at the top-right region of merge unit
                if x_offset > 0 && y_offset > 0 {
                    let mismatch = (self.x_range.1 - x.0) - (self.y_range.1 - x.1);
                    // longer common path 
                    if mismatch < 0 {
                            tap_point.0 = self.x_range.1;
                            tap_point.1 = x.1 - (mismatch/2) as i32;
                            if tap_point.1 > top_right_offset {
                                top_right_offset = tap_point.1;
                            }
                        // add path from sink to tap point, because vertical first
                        self.path.push(Path{from:*x,to:tap_point});
                    }
                    // shorter common path
                    else {
                        tap_point.1 = self.root.1;
                        tap_point.0 = self.x_range.1 - (mismatch/2) as i32;
                            // add path from tap point to sink, because vertical first
                            self.path.push(Path{from:tap_point,to:*x});
                    }
                }  
                // case 2: at the bottom-right region of merge unit
                else if x_offset > 0 && y_offset < 0 {
                    let mismatch = (self.x_range.1 - x.0) - (self.y_range.1 - x.1);
                    // longer common path
                    if mismatch < 0 {
                        tap_point.0 = self.x_range.1;
                        tap_point.1 = x.1 + (mismatch/2) as i32;
                        if tap_point.1 < bottom_right_offset {
                            bottom_right_offset = tap_point.1;
                        }
                        // add path from sink to tap point, because vertical first
                        self.path.push(Path{from:*x,to:tap_point});
                    } 
                    // shorter common path
                    else {
                        tap_point.1 = self.root.1;
                        tap_point.0 = self.x_range.1 - (mismatch/2) as i32;
                                            // add path from tap point to x, because vertical first
                        self.path.push(Path{from:tap_point,to:*x});
                    }
                }
                // case 3: at the top-left region of merge unit
                else if x_offset < 0 && y_offset > 0 {
                    let mismatch = (x.0 - self.x_range.0) - (self.y_range.1 - x.1);
                    // longer common path
                    if mismatch < 0 {
                        tap_point.0 = self.x_range.0;
                        tap_point.1 = x.1 - (mismatch/2) as i32;
                        if tap_point.1 > top_left_offset {
                            top_left_offset = tap_point.1;
                        }
                                                // add path from sink to tap point, because vertical first
                        self.path.push(Path{from:*x,to:tap_point});
                    }
                    // shorter common path 
                    else {
                        tap_point.1 = self.root.1;
                        tap_point.0 = self.x_range.0 + (mismatch/2) as i32;
                                                // add path from tap point to sink, because vertical first
                        self.path.push(Path{from:tap_point,to:*x});
                    }
                }
                // case 4: at the bottom-left region of merge unit
                else if x_offset < 0 && y_offset < 0 {
                    let mismatch = (x.0-self.x_range.0) - (x.1-self.y_range.0);
                    // longer common path
                    if mismatch < 0 {
                        tap_point.0 = self.x_range.0;
                        tap_point.1 = x.1 + (mismatch)/2 as i32;
                        if tap_point.1 < bottom_left_offset {
                            bottom_left_offset = tap_point.1;
                        }
                                                // add path from sink to tap point, because vertical first
                        self.path.push(Path{from:*x,to:tap_point});
                    }
                    // shorter common path
                    else {
                        tap_point.1 = self.root.1;
                        tap_point.0 = self.x_range.0 + (mismatch/2) as i32;
                                                // add path from tap point to sink, because vertical first
                        self.path.push(Path{from:tap_point,to:*x});
                    }
                }
            }
        self.path.push(Path{from:(top_left_offset,self.x_range.0),to:(bottom_left_offset,self.x_range.0)});
        self.path.push(Path{from:(top_right_offset,self.x_range.1),to:(bottom_right_offset,self.x_range.1)});
        self.path.push(Path{from:(self.x_range.0,self.root.1),to:(self.x_range.1,self.root.1)});
        }
        
        
        //
    }
}


