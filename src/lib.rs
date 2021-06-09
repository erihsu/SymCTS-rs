#![allow(dead_code)]

mod cfg;
mod merge;
mod model;
// mod node;
mod error;
mod stage;

use crate::cfg::load_design;
use crate::model::*;

use stage::cfg::*;
use std::error::Error;
use std::fs;
use std::result::Result;

pub fn run_symcts(
    design_cfg_path: &str,
    plugin_cfg_path: &str,
    cts_cfg_path: &str,
) -> Result<(), Box<dyn Error>> {
    let mut my_design = load_design(plugin_cfg_path, design_cfg_path)?;
    let cts_cfg: CtsCfg = serde_yaml::from_str(&fs::read_to_string(cts_cfg_path)?)?;
    let sinks: Vec<(String, (i32, i32))> = my_design.get_clock_sinks("")?;
    let mut clocktree = ClockTree {
        name: cts_cfg.name.clone(),
        ..Default::default()
    };
    let (mut x_min, mut x_max, mut y_min, mut y_max) = (i32::MAX, i32::MIN, i32::MAX, i32::MIN);
    clocktree.sinks = sinks
        .iter()
        .map(|x| {
            let (sink_x, sink_y) = x.1;
            // let (offset_x, offset_y) =
            //     *sink_offset.get(&x.0).and_then(|x| x.get(&orient)).unwrap();
            // let offset_dbu = (
            //     (dbu_factor * offset_x) as i32,
            //     (dbu_factor * offset_y) as i32,
            // );
            // // let load_cap = *sink_cap.get(&x.0).unwrap();
            // let sink_x = origin_x + offset_dbu.0;
            // let sink_y = origin_y + offset_dbu.1;
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
                location: (sink_x, sink_y),
            }
        })
        .collect();
    clocktree.x_range = (x_min, x_max);
    clocktree.y_range = (y_min, y_max);
    println!("x_range:{:?}", clocktree.x_range);
    println!("y_range:{:?}", clocktree.y_range);
    println!("Load CTS related data successfully");

    clocktree.gen_topology(&cts_cfg.stage1_cfg);
    clocktree.buffering(&cts_cfg.stage2_cfg);

    // exporting result

    Ok(())
}
