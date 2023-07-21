use std::collections::HashMap;
use std::str;

use crate::term::*;
use crate::hint::*;

pub struct TermCtx<'a> {
    pub output : Vec<u8>,
    pub current : String,
    pub args : Vec<String>,
    pub chint : Option<ExcerptIter<'a, String>>,
    pub hints : &'a ShCommands
}

impl<'a> DefaultVal for TermCtx<'a> {
    fn val(&self) -> &Vec<String> {
        &self.args
    }
}

impl<'a> TermCtx<'a> {
    fn new(hints: &'a ShCommands) -> Self { 
        TermCtx{
            output: vec![],
            current: String::from(""),
            args: vec![],
            chint: None,
            hints: hints
        }
    }

    pub fn pushstr(&mut self) {
        if self.current.len() > 0 {
            self.args.push(self.current.clone());
            self.current = String::from("");
        }
    }

    pub fn autocomplete(&mut self) {
        if self.current.len() > 0 {
            match &self.chint {
                Some(ch) => {
                    match ch.peek() {
                        Some(chint) => {
                            Term.hmove((chint.len() - self.current.len() + 1) as i32);
                            self.current = chint.to_string();
                            self.pushstr();
                        },
                        None => ()
                    }
                },
                None => ()
            }
        }
    }
}

const BACKSPACE: u8 = 0x7f;
const CTRL_D: u8 = 0x04;

fn more_keys<'a>() -> KeyBind<TermCtx<'a>> {
    HashMap::from([
        (b' ', KeyAction::Action(ac_space)),
        (b'\n', KeyAction::Action(ac_ret)),
        (BACKSPACE, KeyAction::Action(ac_bs)),
        (b'\t', KeyAction::Action(quit_cmd)),
        (CTRL_D, KeyAction::Action(terminate))
    ])
}

fn initial_keys<'a>() -> KeyBind<TermCtx<'a>> {
    HashMap::from(
        [(b'\n', KeyAction::Action(send_output)),
         (b'\t', KeyAction::Action(enter_cmd)),
         (BACKSPACE, KeyAction::Action(ac_min_bs)),
         (CTRL_D, KeyAction::Action(terminate))
    ])
}

pub type MyReader<'a> = TermReader<TermCtx<'a>>;

pub fn default_term<'a,'b>(hints: &'b ShCommands) -> MyReader<'a>
        where 'b:'a {
    MyReader::new(TermCtx::new(hints),
                  initial_keys(), KeyAction::Action(out_elsekey))
}

fn enter_cmd<'a>(tr: &mut MyReader<'a>, _keys: &[u8]) -> Reading {
    tr.set_mapping(more_keys(), KeyAction::Action(cmd_elsekey));
    Reading::tbc(None)
}

fn terminate<'a>(_tr: &mut MyReader<'a>, _keys: &[u8]) -> Reading {
    Reading::finished(None)
}

fn send_output<'a>(tr: &mut MyReader<'a>, _keys: &[u8]) -> Reading {
    let mut out = tr.ctx.output.clone();
    out.push(b'\n');
    Term.echo(b"\n");
    tr.ctx.output = vec![];
    Reading::tbc(Some(out))
}

fn quit_cmd<'a>(tr: &mut MyReader<'a>, _keys: &[u8]) -> Reading {
    tr.set_mapping(initial_keys(), KeyAction::Action(out_elsekey));
    Reading::tbc(None)
}

fn cmd_elsekey<'a> (tr: &mut MyReader<'a>, keys: &[u8]) -> Reading {
    if tr.ctx.args.len() > 0 {
        echo(keys);
        tr.ctx.current.push_str(str::from_utf8(keys).unwrap());
        return Reading::tbc(None)
    }

    let trial = tr.ctx.current.clone() + str::from_utf8(keys).unwrap();
    match tr.ctx.hints.for_prefix(&trial) {
        Some(mut it) => {
            let first = it.get().unwrap();
            match first.get(tr.ctx.current.len()..) {
                Some(s) => {
                    Term.echo(s.as_bytes())
                        .endline()
                        .move_left(s.len() - 1);
                }, None => ()
            }
    
            tr.ctx.current = trial;
            tr.ctx.chint = Some(it);
            Reading::tbc(None)
        },
        None => Reading::tbc(None)
    }
}

fn out_elsekey<'a>(tr: &mut MyReader<'a>, keys: &[u8]) -> Reading {
    echo(keys);
    tr.ctx.output.extend_from_slice(keys);
    Reading::tbc(None)
}

fn ac_min_bs<'a> (tr: &mut MyReader<'a>, _keys: &[u8]) -> Reading {
    Term.hmove(-1);
    tr.ctx.output.pop();
    Reading::tbc(None)
}

fn ac_bs<'a> (tr: &mut MyReader<'a>, _keys: &[u8]) -> Reading {
    if tr.ctx.args.len() > 0 {
        Term.hmove(-1);
        tr.ctx.current.pop();
        return Reading::tbc(None)
    }

    if tr.ctx.current.len() == 0 {
        return Reading::tbc(None);
    }

    let mut trial = tr.ctx.current.clone();
    trial.pop();
    match tr.ctx.hints.for_prefix(&trial) {
        Some(mut it) => {
            let first = it.get().unwrap();
            let term = Term;
            term.hmove(-1);
            match first.get(trial.len()..) {
                Some(s) => { term.echo(s.as_bytes())
                                 .endline()
                                 .hmove(-(s.len() as i32)); }
                None => ()
            }
   
            tr.ctx.current = trial;
            tr.ctx.chint = Some(it);
            Reading::tbc(None)
        },
        None => Reading::tbc(None)
    }
}

fn ac_space<'a> (tr: &mut MyReader<'a>, keys: &[u8]) -> Reading {
    if tr.ctx.args.len() > 0 {
        if tr.ctx.current.len() > 0 {
            echo(keys);
            tr.ctx.pushstr();
            return Reading::tbc(None);
        }
        return Reading::tbc(None)
    }

    tr.ctx.autocomplete();
    Reading::tbc(None)
}

fn ac_ret<'a> (tr: &mut MyReader<'a>, _: &[u8]) -> Reading {
    if tr.ctx.args.len() > 0 {
        if tr.ctx.current.len() > 0 {
            tr.ctx.pushstr();
            return Reading::finished(None);
        }
        return Reading::finished(None)
    }

    Term.echo(b"\n");
    
    tr.ctx.autocomplete();
    println!("{:?}", tr.ctx.val());
    Reading::finished(None)
}

