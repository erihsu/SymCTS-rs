// try to implement manhattan style merge that given set of point, get the
// parent node which promise least total wire source and equal manhattan distance
// to each child node
// Note: there is no limit on number of given points, that is , it support
// non-binary tree merge
use cts_plugin::Path;

#[derive(Default)]
pub struct MergeUnit {
    sink: Vec<(i32, i32)>,
    pub root: (i32, i32),
    x_range: (i32, i32),
    y_range: (i32, i32),
    common_length: u32,
    pub path: Vec<Path>,
    if_horizontal: bool, // merge unit direction
    pub level: u8,
}

impl MergeUnit {
    pub fn new() -> Self {
        Default::default()
    }
    pub fn is_root(&self) -> bool {
        self.level == 1
    }
    pub fn length(&self) -> i32 {
        self.path.iter().fold(0, |acc, x| acc + x.length())
    }
    pub fn set_level(&mut self, level: u8) {
        self.level = level;
    }
    pub fn adjust_root(&mut self, margin: u32) {
        if self.if_horizontal {
            let new_root = if self.root.1 - self.y_range.0 > self.y_range.1 - self.root.1 {
                (self.root.0, self.root.1 + margin as i32)
            } else {
                (self.root.1, self.root.1 + margin as i32)
            };
            self.path.push(Path {
                from: self.root,
                turn: None,
                to: new_root,
            });
            self.root = new_root;
        } else {
            let new_root = if self.root.0 - self.x_range.0 > self.x_range.1 - self.root.0 {
                (self.root.0 - margin as i32, self.root.1)
            } else {
                (self.root.0 + margin as i32, self.root.1)
            };
            self.path.push(Path {
                from: self.root,
                turn: None,
                to: new_root,
            });
            self.root = new_root;
        }
    }
    pub fn common_length(&self) -> u32 {
        self.common_length
    }
    // load sink , analyze range, get root and common length
    pub fn load_sink(&mut self, sink: &[(i32, i32)]) {
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
            self.sink.push(*d);
        });
        self.x_range = (x_min, x_max);
        self.y_range = (y_min, y_max);

        // analyze
        if (self.x_range.0 - self.x_range.1).abs() > (self.y_range.0 - self.y_range.1).abs() {
            self.if_horizontal = true;
            self.root = ((lm.0 + rm.0) / 2, (lm.1 + rm.1) / 2) as (i32, i32);
            self.common_length = ((lm.0 - self.root.0).abs() + (lm.1 - self.root.1).abs()) as u32;
        } else {
            self.if_horizontal = false;
            self.root = ((bm.0 + tm.0) / 2, (bm.1 + tm.1) / 2) as (i32, i32);
            self.common_length = ((bm.0 - self.root.0).abs() + (bm.1 - self.root.1).abs()) as u32;
        }
    }

    pub fn merge(&mut self) {
        if self.if_horizontal {
            for x in &self.sink {
                if x.0 == self.x_range.0 || x.0 == self.x_range.1 {
                    let turn = (x.0, self.root.1);
                    self.path.push(Path {
                        from: *x,
                        turn: Some(turn),
                        to: self.root,
                    });
                } else {
                    let end = (x.0, self.root.1);
                    self.path.push(Path {
                        from: *x,
                        turn: None,
                        to: end,
                    });
                }
            }
        } else {
            for x in &self.sink {
                if x.1 == self.y_range.0 || x.1 == self.y_range.1 {
                    let turn = (self.root.0, x.1);
                    self.path.push(Path {
                        from: *x,
                        turn: Some(turn),
                        to: self.root,
                    });
                } else {
                    let end = (self.root.0, x.1);
                    self.path.push(Path {
                        from: *x,
                        turn: None,
                        to: end,
                    });
                }
            }
        }
    }

    //     pub fn merge(&mut self) {
    //         if self.if_horizontal {
    //             for x in &self.sink {
    //                 if x.0 == self.x_range.0 || x.0 == self.x_range.1 {
    //                     let turn = (x.0,self.root.1);
    //                     self.path.push(Path{from:*x,turn:Some(turn),to:self.root});
    //                 } else {
    //                     let sink2root = ((x.0 - self.root.0).abs() + (x.1 - self.root.1).abs()) as u32;
    //                     let delta = self.common_length as i32 - sink2root as i32;

    //                     let mut end = x.clone();
    //                     if delta > 0 {
    //                         let extra = delta/2;
    //                         if x.0 <= self.root.0 {
    //                             end.0 -= extra as i32;
    //                             end.1 = self.root.1;
    //                         } else {
    //                             end.0 += extra as i32;
    //                             end.1 = self.root.1;
    //                         }
    //                         let turn = (end.0,x.1);
    //                         self.path.push(Path{from:*x,turn:Some(turn),to:end});
    //                     } else {
    //                         // prepare for new path
    //                         let start = end;
    //                         let end = (start.0,self.root.1);
    //                         self.path.push(Path{from:start,turn:None,to:end});
    //                     }

    //                 }
    //             }

    //         }
    //         else {
    //             for x in &self.sink {
    //                 if x.1 == self.y_range.0 || x.1 == self.y_range.1 {
    //                     let turn = (self.root.0,x.1);
    //                     self.path.push(Path{from:*x,turn:Some(turn),to:self.root});
    //                 } else {
    //                     let sink2root = ((x.0 - self.root.0).abs() + (x.1 - self.root.1).abs()) as u32;
    //                     let delta = self.common_length as i32 - sink2root as i32;
    //                     let mut end = x.clone();
    //                     if delta > 0 {
    //                         let extra = delta/2;
    //                         if x.1 <= self.root.1 {
    //                             end.1 -= extra as i32;
    //                             end.0 = self.root.0;
    //                         } else {
    //                             end.1 += extra as i32;
    //                             end.0 = self.root.0;
    //                         }
    //                         let turn = (x.0,end.1);
    //                         self.path.push(Path{from:*x,turn:Some(turn),to:end});
    //                     } else {
    //                         let start = end;
    //                         let end = (self.root.0,start.1);
    //                         self.path.push(Path{from:start,turn:None,to:end});
    //                     }

    //                 }
    //             }

    //         }

    // }
}
