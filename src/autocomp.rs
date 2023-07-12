use std::collections::HashMap;
use std::str;

use crate::term::*;
use crate::hint::*;

fn more_keys() -> KeyBind {
    HashMap::from([
        (b' ', KeyAction::Action(ac_space)),
        (b'\n', KeyAction::Action(ac_ret)),
        (0x7f, KeyAction::Action(ac_bs)),
        (b'\t', KeyAction::Action(quit_cmd))
    ])
}

fn initial_keys() -> KeyBind {
    HashMap::from(
        [(b'\n', KeyAction::Action(send_output)),
         (b'\t', KeyAction::Action(enter_cmd)),
         (0x7f, KeyAction::Action(ac_min_bs))])
}

pub fn default_term<'a>() -> TermReader<'a> {
    TermReader::new(initial_keys(), KeyAction::Action(out_elsekey))
}

fn enter_cmd<'a>(tr: TermReader<'a>, _hints: &'a ShCommands, _keys: &[u8]) -> Reading<'a> {
    Reading::tbc(TermReader {
            output: tr.output,
            current: tr.current,
            chint: tr.chint,
            args: tr.args,
            key_map: more_keys(),
            elsekey: KeyAction::Action(cmd_elsekey)
        }, None)
}

fn send_output<'a>(tr: TermReader<'a>, _hints: &'a ShCommands, _keys: &[u8]) -> Reading<'a> {
    let mut out = tr.output;
    out.push_str("\n");
    Term.echo(b"\n");
    Reading::tbc(TermReader {
            output: String::from(""),
            current: tr.current,
            chint: tr.chint,
            args: tr.args,
            key_map: tr.key_map,
            elsekey: tr.elsekey
        }, Some(out))
}

fn quit_cmd<'a>(tr: TermReader<'a>, _hints: &'a ShCommands, _keys: &[u8]) -> Reading<'a> {
    Reading::tbc(TermReader {
            output: tr.output,
            current: tr.current,
            chint: tr.chint,
            args: tr.args,
            key_map: initial_keys(),
            elsekey: KeyAction::Action(out_elsekey)
        }, None)
}

fn cmd_elsekey<'a> (tr: TermReader<'a>, hints: &'a ShCommands, keys: &[u8]) -> Reading<'a> {
    if tr.args.len() > 0 {
        echo(keys);
        let nc = tr.current.clone() + str::from_utf8(keys).unwrap();
        return Reading::tbc(tr.with_current(nc), None)
    }

    let trial = tr.current.clone() + str::from_utf8(keys).unwrap();
    match hints.for_prefix(&trial) {
        Some(mut it) => {
            let first = it.get().unwrap();
            match first.get(tr.current.len()..) {
                Some(s) => {
                    Term.echo(s.as_bytes())
                        .endline()
                        .move_left(s.len() - 1);
                }, None => ()
            }
    
            Reading::tbc(TermReader {
                    output: tr.output,
                    current: trial,
                    chint: Some(it),
                    args: tr.args,
                    key_map: tr.key_map,
                    elsekey: tr.elsekey
                }, None)
        },
        None => Reading::tbc(tr, None)
    }
}

fn out_elsekey<'a>(tr: TermReader<'a>, _hints: &'a ShCommands, keys: &[u8]) -> Reading<'a> {
    echo(keys);
    let mut out = tr.output;
    out.push_str(str::from_utf8(keys).unwrap());
    Reading::tbc(TermReader {
            output: out,
            current: tr.current,
            chint: tr.chint,
            args: tr.args,
            key_map: tr.key_map,
            elsekey: tr.elsekey
        }, None)
}

fn ac_min_bs<'a> (tr: TermReader<'a>, _hints: &'a ShCommands, _keys: &[u8]) -> Reading<'a> {
    Term.hmove(-1);
    let mut out = tr.output;
    out.pop();
    Reading::tbc(TermReader {
            output: out,
            current: tr.current,
            chint: tr.chint,
            args: tr.args,
            key_map: tr.key_map,
            elsekey: tr.elsekey
        }, None)
}

fn ac_bs<'a> (tr: TermReader<'a>, hints: &'a ShCommands, _keys: &[u8]) -> Reading<'a> {
    if tr.args.len() > 0 {
        Term.hmove(-1);
        let mut nc = tr.current.clone();
        nc.pop();
        return Reading::tbc(tr.with_current(nc), None)
    }

    if tr.current.len() == 0 {
        return Reading::tbc(tr, None);
    }

    let mut trial = tr.current.clone();
    trial.pop();
    match hints.for_prefix(&trial) {
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
                    output: tr.output,
                    current: trial,
                    chint: Some(it),
                    args: tr.args,
                    elsekey: tr.elsekey,
                    key_map: tr.key_map
                }, None)
        },
        None => Reading::tbc(tr, None)
    }
}

fn ac_space<'a> (tr: TermReader<'a>, _hints: &'a ShCommands,  keys: &[u8]) -> Reading<'a> {
    if tr.args.len() > 0 {
        if tr.current.len() > 0 {
            echo(keys);
            return Reading::tbc(tr.pushstr(), None);
        }
        return Reading::tbc(tr, None)
    }

    Reading::tbc(tr.autocomplete(), None)
}

fn ac_ret<'a> (tr: TermReader<'a>, _hints: &'a ShCommands, _: &[u8]) -> Reading<'a> {
    if tr.args.len() > 0 {
        if tr.current.len() > 0 {
            return Reading::finished(tr.pushstr(), None);
        }
        return Reading::finished(tr, None)
    }

    let ntr = tr.autocomplete();
    Term.echo(b"\n");

    Reading::finished(ntr, None)
}

