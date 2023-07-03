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
        [(b'\t', KeyAction::Action(enter_cmd)),
         (0x7f, KeyAction::Action(ac_min_bs))])
}

pub fn default_term<'a>() -> TermReader<'a> {
    TermReader::new(initial_keys(), None)
}

fn enter_cmd<'a>(tr: TermReader<'a>, _hints: &'a ShCommands, _keys: &[u8]) -> (bool, TermReader<'a>) {
    (true, TermReader {
        current: tr.current,
        chint: tr.chint,
        args: tr.args,
        key_map: more_keys(),
        elsekey: Some(KeyAction::Action(ac_elsekey))
    })
}

fn quit_cmd<'a>(tr: TermReader<'a>, _hints: &'a ShCommands, _keys: &[u8]) -> (bool, TermReader<'a>) {
    (true, TermReader {
        current: tr.current,
        chint: tr.chint,
        args: tr.args,
        key_map: initial_keys(),
        elsekey: None
    })
}

fn ac_elsekey<'a> (tr: TermReader<'a>, hints: &'a ShCommands, keys: &[u8]) -> (bool, TermReader<'a>) {
    if tr.args.len() > 0 {
        echo(keys);
        let nc = tr.current.clone() + str::from_utf8(keys).unwrap();
        return (true, tr.with_current(nc))
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
    
            (true, TermReader {
                current: trial,
                chint: Some(it),
                args: tr.args,
                key_map: tr.key_map,
                elsekey: tr.elsekey
            })
        },
        None => (true, tr)
    }
}

fn ac_min_bs<'a> (tr: TermReader<'a>, _hints: &'a ShCommands, _keys: &[u8]) -> (bool, TermReader<'a>) {
    Term.hmove(-1);
    (true, tr)
}

fn ac_bs<'a> (tr: TermReader<'a>, hints: &'a ShCommands, _keys: &[u8]) -> (bool, TermReader<'a>) {
    if tr.args.len() > 0 {
        Term.hmove(-1);
        let mut nc = tr.current.clone();
        nc.pop();
        return (true, tr.with_current(nc))
    }

    if tr.current.len() == 0 {
        return (true, tr);
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
    
            (true, TermReader {
                current: trial,
                chint: Some(it),
                args: tr.args,
                elsekey: tr.elsekey,
                key_map: tr.key_map
            })
        },
        None => (true, tr)
    }
}

fn ac_space<'a> (tr: TermReader<'a>, _hints: &'a ShCommands,  keys: &[u8]) -> (bool,TermReader<'a>) {
    if tr.args.len() > 0 {
        if tr.current.len() > 0 {
            echo(keys);
            return (true, tr.pushstr());
        }
        return (true, tr)
    }

    (true, tr.autocomplete())
}

fn ac_ret<'a> (tr: TermReader<'a>, _hints: &'a ShCommands, _: &[u8]) -> (bool, TermReader<'a>) {
    if tr.args.len() > 0 {
        if tr.current.len() > 0 {
            return (false, tr.pushstr());
        }
        return (false, tr)
    }

    let ntr = tr.autocomplete();
    Term.echo(b"\n");

    (false,ntr)
}

