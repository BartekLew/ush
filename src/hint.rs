use std::fs::*;
use std::iter::*;

pub struct ShCommands {
    cmds : Vec<String>,
}

impl ShCommands {
    pub fn new() -> Self {
        let mut cmds = std::env::var("PATH")
                        .unwrap_or("/bin:/usr/bin:/sbin:/usr/sbin".to_string())
                        .split(":")
                        .map(|dir| match read_dir(dir) {
                                    Ok(d) => Ok(d.map(|res| res.unwrap()
                                                               .file_name()
                                                               .into_string()
                                                               .unwrap())),
                                    Err(e) => Err(format!("{}: {}", dir, e))
                        })
                        .filter(|res| match res {
                            Ok(_) => true,
                            Err(e) => {
                                    println!("Warning: {}", e);
                                    false
                            }})
                        .flat_map(|res| res.unwrap())
                        .collect::<Vec<String>>();
        cmds.sort_by(|a, b| a.to_lowercase().cmp(&b.to_lowercase()));
        cmds.dedup();
        ShCommands { cmds: cmds }
    }

    pub fn for_prefix(&self, prefix: &String) -> Vec<&String> {
        self.cmds.iter()
                 .skip_while(|s| !prefix_eq(s, prefix))
                 .take_while(|s| prefix_eq(s, prefix))
                 .collect()
    }
}

fn prefix_eq(str:&String, prefix:&String) -> bool {
    match str.get(0..prefix.len()) {
        Some(substr) => substr.eq(prefix),
        None => false
    }
}
