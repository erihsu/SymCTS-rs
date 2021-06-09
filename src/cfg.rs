use cts_plugin::CTSPlugin;
use libloading::Library;
use serde::{Deserialize, Serialize};
use std::error::Error;
#[derive(Serialize, Deserialize)]
struct DesignCfg {
    verilog_path: String,
    def_path: String,
}

impl DesignCfg {
    pub fn new<P: AsRef<std::path::Path>>(
        path: P,
    ) -> std::result::Result<DesignCfg, Box<dyn Error>> {
        let cfg_str = std::fs::read_to_string(path)?;
        let design_cfg: DesignCfg = serde_yaml::from_str(&cfg_str)?;
        Ok(design_cfg)
    }
}

#[derive(Serialize, Deserialize)]
struct PluginCfg {
    plugin_path: String,
    username: String,
    password: String,
}

impl PluginCfg {
    pub fn new<P: AsRef<std::path::Path>>(
        path: P,
    ) -> std::result::Result<Box<dyn CTSPlugin>, Box<dyn Error>> {
        let cfg_str = std::fs::read_to_string(path)?;
        let plg_cfg: PluginCfg = serde_yaml::from_str(&cfg_str)?;
        let cts_lib = Library::new(&plg_cfg.plugin_path)?;
        let new_design_plugin: libloading::Symbol<fn() -> Box<dyn CTSPlugin>> =
            unsafe { cts_lib.get(b"new_design_plugin") }?;
        // allocate
        let mut design_plugin = new_design_plugin();
        design_plugin.login(&plg_cfg.username, &plg_cfg.password)?;
        println!(
            "Successfully login in to Vulcan PDK database with {}",
            &plg_cfg.username
        );
        Ok(design_plugin)
    }
}

pub fn load_design<P: AsRef<std::path::Path>>(
    p1: P,
    p2: P,
) -> std::result::Result<Box<dyn CTSPlugin>, Box<dyn Error>> {
    let mut plg = PluginCfg::new(p1)?;
    let design = DesignCfg::new(p2)?;
    plg.import_verilog(&design.verilog_path)?;
    plg.import_def(&design.def_path)?;
    Ok(plg)
}
