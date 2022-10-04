#ifndef __H_TERM
#define __H_TERM 1

#include "misc.h"

void termcur_hmove(int n);
void term_endline();
void term_delchar(int count);
void term_backspace();

#endif
