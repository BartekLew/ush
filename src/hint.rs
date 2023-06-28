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

    pub fn for_prefix(&self, prefix: &String) -> Option<ExcerptIter<String>>{
        let mut it = self.cmds.iter().enumerate().peekable();
        while match it.peek() {
                Some(x) => !prefix_eq(x.1, prefix),
                None => return None
              } { it.next(); }

        let s = it.peek().unwrap().0;
        while match it.peek() {
            Some(x) => prefix_eq(x.1, prefix),
            None => return Some(ExcerptIter::new(&self.cmds, s, self.cmds.len()).unwrap())
        } { it.next(); }

        let e = it.peek().unwrap().0;

        Some(ExcerptIter::new(&self.cmds, s, e).unwrap())
    }
}

fn prefix_eq(str:&String, prefix:&String) -> bool {
    match str.get(0..prefix.len()) {
        Some(substr) => substr.eq(prefix),
        None => false
    }
}

pub struct ExcerptIter<'a, T> {
    start : usize,
    end: usize,
    pos: Option<usize>,
    subject: &'a Vec<T>
}

impl<'a, T> ExcerptIter<'a, T> {
    pub fn new(subject: &'a Vec<T>, start: usize, end: usize) -> Result<Self,String> {
        match start < end && end <= subject.len() {
            true => Ok(ExcerptIter { start: start,
                                     end: end,
                                     pos: None,
                                     subject: subject }),
            false => Err(format!("Wrong range {}:{}; subject length is {}.", start, end, subject.len()))
        }
    }

    fn get_offset(&mut self, off: i32) -> Option<&T> {
        match self.pos {
            Some(pos) => { let newpos = pos as i64 + off as i64;
                           match newpos > 0 && newpos < self.end as i64 {
                                true => {
                                    self.pos = Some(newpos as usize);
                                    Some(&self.subject[newpos as usize])
                                },
                                false => None
                            }
                        },
            None => {
                self.pos = Some(self.start);
                Some(&self.subject[self.start])
            }
        }
    }

//    pub fn len(&self) -> usize {
//        self.end - self.start
//    }

    pub fn get(&mut self) -> Option<&T> {
        self.get_offset(0)
    }

    pub fn peek(&self) -> Option<&T> {
        match self.pos {
            Some (p) => Some(&self.subject[p]),
            None => None
        }
    }

//    pub fn next(&mut self) -> Option<&T> {
//        self.get_offset(1)
//    }
//    pub fn prev(&mut self) -> Option<&T> {
//        self.get_offset(-1)
//    }
}

#[test]
fn iterator_next_prev() {
    let iterated = vec!["foo", "bar", "baz", "um", "tum"];
    let mut sut = ExcerptIter::new(&iterated, 1, 3).unwrap();

    assert_eq!(sut.next().unwrap().to_string(), "bar");
    assert_eq!(sut.next().unwrap().to_string(), "baz");
    assert!(sut.next().is_none());
    assert_eq!(sut.prev().unwrap().to_string(), "bar");
    assert!(sut.prev().is_none());
}
