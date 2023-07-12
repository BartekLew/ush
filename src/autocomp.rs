use std::collections::HashMap;
use std::str;

use crate::term::*;
use crate::hint::*;

pub struct TermCtx<'a> {
    pub output : String,
    pub current : String,
    pub args : Vec<String>,
    pub chint : Option<ExcerptIter<'a, String>>,
    pub hints : &'a ShCommands
}

impl<'a> DefaultVal for TermCtx<'a> {
    fn val(self) -> Vec<String> {
        self.args
    }
}

impl<'a> TermCtx<'a> {
    fn new(hints: &'a ShCommands) -> Self { 
        TermCtx{
            output: String::from(""),
            current: String::from(""),
            args: vec![],
            chint: None,
            hints: hints
        }
    }

    pub fn pushstr(mut self) -> TermCtx<'a> {
        if self.current.len() > 0 {
            self.args.push(self.current.clone());
            self.current = String::from("");
        }
        self
    }

    pub fn autocomplete(mut self) -> TermCtx<'a> {
        if self.current.len() > 0 {
            match &self.chint {
                Some(ch) => {
                    match ch.peek() {
                        Some(chint) => {
                            Term.hmove((chint.len() - self.current.len() + 1) as i32);
                            self.current = chint.to_string();
                            self.pushstr()
                        },
                        None => self
                    }
                },
                None => self
            }
        } else {
            self
        }
    }
}

fn more_keys<'a>() -> KeyBind<TermCtx<'a>> {
    HashMap::from([
        (b' ', KeyAction::Action(ac_space)),
        (b'\n', KeyAction::Action(ac_ret)),
        (0x7f, KeyAction::Action(ac_bs)),
        (b'\t', KeyAction::Action(quit_cmd))
    ])
}

fn initial_keys<'a>() -> KeyBind<TermCtx<'a>> {
    HashMap::from(
        [(b'\n', KeyAction::Action(send_output)),
         (b'\t', KeyAction::Action(enter_cmd)),
         (0x7f, KeyAction::Action(ac_min_bs))])
}

pub type MyReader<'a> = TermReader<TermCtx<'a>>;

pub fn default_term<'a>(hints: &'a ShCommands) -> MyReader<'a> {
    MyReader::new(TermCtx::new(hints),
                  initial_keys(), KeyAction::Action(out_elsekey))
}

fn enter_cmd<'a>(tr: MyReader<'a>, _keys: &[u8]) -> Reading<TermCtx<'a>> {
    Reading::tbc(tr.with_mapping(more_keys(), KeyAction::Action(cmd_elsekey)), None)
}

fn send_output<'a>(tr: MyReader<'a>, _keys: &[u8]) -> Reading<TermCtx<'a>> {
    let mut out = tr.ctx.output;
    out.push_str("\n");
    Term.echo(b"\n");
    Reading::tbc(TermReader{ ctx: TermCtx {
                                output: String::from(""),
                                current: tr.ctx.current,
                                hints: tr.ctx.hints,
                                chint: tr.ctx.chint,
                                args: tr.ctx.args
                             },
                             key_map: tr.key_map, elsekey: tr.elsekey },
                 Some(out))
}

fn quit_cmd<'a>(tr: MyReader<'a>, _keys: &[u8]) -> Reading<TermCtx<'a>> {
    Reading::tbc(tr.with_mapping(initial_keys(), KeyAction::Action(out_elsekey)), None)
}

fn cmd_elsekey<'a> (mut tr: MyReader<'a>, keys: &[u8]) -> Reading<TermCtx<'a>> {
    if tr.ctx.args.len() > 0 {
        echo(keys);
        tr.ctx.current.push_str(str::from_utf8(keys).unwrap());
        return Reading::tbc(tr, None)
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
    
            Reading::tbc(TermReader {
                    ctx: TermCtx {
                        output: tr.ctx.output,
                        current: trial,
                        chint: Some(it),
                        args: tr.ctx.args,
                        hints: tr.ctx.hints
                    },
                    key_map: tr.key_map,
                    elsekey: tr.elsekey
                }, None)
        },
        None => Reading::tbc(tr, None)
    }
}

fn out_elsekey<'a>(mut tr: MyReader<'a>, keys: &[u8]) -> Reading<TermCtx<'a>> {
    echo(keys);
    tr.ctx.output.push_str(str::from_utf8(keys).unwrap());
    Reading::tbc(tr, None)
}

fn ac_min_bs<'a> (mut tr: MyReader<'a>, _keys: &[u8]) -> Reading<TermCtx<'a>> {
    Term.hmove(-1);
    tr.ctx.output.pop();
    Reading::tbc(tr, None)
}

fn ac_bs<'a> (mut tr: MyReader<'a>, _keys: &[u8]) -> Reading<TermCtx<'a>> {
    if tr.ctx.args.len() > 0 {
        Term.hmove(-1);
        tr.ctx.current.pop();
        return Reading::tbc(tr, None)
    }

    if tr.ctx.current.len() == 0 {
        return Reading::tbc(tr, None);
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
    
            Reading::tbc(TermReader {
                    ctx: TermCtx {
                        output: tr.ctx.output,
                        current: trial,
                        chint: Some(it),
                        args: tr.ctx.args,
                        hints: tr.ctx.hints
                    },
                    elsekey: tr.elsekey,
                    key_map: tr.key_map
                }, None)
        },
        None => Reading::tbc(tr, None)
    }
}

fn ac_space<'a> (tr: MyReader<'a>, keys: &[u8]) -> Reading<TermCtx<'a>> {
    if tr.ctx.args.len() > 0 {
        if tr.ctx.current.len() > 0 {
            echo(keys);
            return Reading::tbc(tr.with_ctx(&|ctx:TermCtx<'a>| ctx.pushstr()), None);
        }
        return Reading::tbc(tr, None)
    }

    Reading::tbc(tr.with_ctx(|ctx| ctx.autocomplete()), None)
}

fn ac_ret<'a> (tr: MyReader<'a>, _: &[u8]) -> Reading<TermCtx<'a>> {
    if tr.ctx.args.len() > 0 {
        if tr.ctx.current.len() > 0 {
            return Reading::finished(tr.with_ctx(|ctx| ctx.pushstr()), None);
        }
        return Reading::finished(tr, None)
    }

    Term.echo(b"\n");

    Reading::finished(tr.with_ctx(|ctx| ctx.autocomplete()), None)
}

