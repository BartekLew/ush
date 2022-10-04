#include "term.h"

void termcur_hmove(int n) {
    if(n != 0) {
        char seq[10];
        int l;
        if(n>0)
            l = sprintf(seq, "\x1b[%dC", n);
        else
            l = sprintf(seq, "\x1b[%dD", -n);
        write(STDOUT_FILENO, seq, l);
    }
}

void term_endline() {
    write(STDOUT_FILENO, "\x1b[K", 3);
}

void term_delchar(int count) {
    if(count > 0) {
        char seq[10];
        int l = sprintf(seq, "\x1b[%dP", count);
        write(STDOUT_FILENO, seq, l);
    }
}

void term_backspace() {
    write(STDOUT_FILENO, "\b\x1b[P", 4);
}
