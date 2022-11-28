#include "prompt.h"
#include "cmdhint.h"
#include "term.h"

PTY *args_to_pty (char *const args[], PTY *ptys) {
    if(args[1] == NULL) {
        fprintf(stderr, "cat: needed PTY number\n");
        return NULL;
    }

    int ptyi;
    if(sscanf(args[1], "%d", &ptyi) != 1 || !PTY_ISOK(ptys[ptyi])) {
        fprintf(stderr, "cat: wrong PTY id: %s\n", args[1]);
        return NULL;
    }

    return ptys + ptyi;
}

volatile bool has_sigstop = false;

bool fg_filter(const char *buff, int size) {
    UNUSED(buff);
    UNUSED(size);
    bool ans = has_sigstop;
    has_sigstop = false;
    return ans;
}

int pty_foreground(PTY *pty) {
    pty->out_bth->canceled = 1;
    write(STDOUT_FILENO, pty->out_bth->buff.data, pty->out_bth->buff.pos);

    if(pty->suspended) {
        kill(pty->pid, SIGCONT);
        pty->suspended = false;
    }

    int deadmask = rewrite_fds(2, (int[]) { STDIN_FILENO, pty->master },
                                  (int[]) { pty->master, STDOUT_FILENO },
                               0x02, &fg_filter);

    int status;
    if(deadmask != 0) {
        waitpid(pty->pid, &status, 0);
        return status;
    }

    return STOP_DEADMASK;
}

int pty_readkey() {
    int c;
    while(read(STDIN_FILENO, &c, 4) > 0 && c != CTRL_D) {
        printf("0x%.8x\r", c);
        fflush(stdout);
        c = 0;
    }

    return 0;
}

char buff[BUFF_SIZE+1];
int buff_pos = 0;

#define RRS_BACKSPACE 0x01
#define RRS_EOL 0x02
#define RRS_EOF 0x04
#define RRS_OVERFLOW 0x08
#define RRS_EOW 0x10
#define RRS_NOP 0x00

typedef struct {
    char *text;
    int  status;
} ReadResult;

typedef struct ii InputInterface;

typedef struct {
    char key;
    int (*handler)(int fd, InputInterface *ii);
} PromptKey;

struct ii {
    PromptKey *keymap;
    size_t    keymap_len;
    int       (*else_handler) (char key, int fd, InputInterface *ii);
    void      (*continue_arg) (InputInterface *ii);
    int       arg_start, word_start;
    ConstStr  promptString;
    CmdHint   *cmdhint;
};

int pk_eof(int fd, InputInterface *ii) {
    UNUSED(fd);
    UNUSED(ii);
    return RRS_EOF;
}

int pk_esc(int fd, InputInterface *ii) {
    UNUSED(ii);
    char cmd;
    if(read(fd, &cmd, 1) == 1) { 
        if(cmd == 0x5b) { // Move Keys
            read(fd, &cmd, 1);
        } 
    }

    return RRS_NOP;
}

static void set_hint(ConstStr new_hint, InputInterface *ii) {
    if(new_hint.str != NULL) {
        size_t pos = buff_pos - ii->word_start;
        strncpy(buff + ii->word_start, new_hint.str, pos);

        termcur_hmove(-ii->cmdhint->prefix_len);
        writestr(STDOUT_FILENO, new_hint);
        term_endline();
        termcur_hmove(pos - new_hint.len);
    }
}

int pkac_esc(int fd, InputInterface *ii) {
    UNUSED(ii);
    char cmd;
    if(read(fd, &cmd, 1) == 1) { 
        if(cmd == 0x5b) { // Move Keys
            read(fd, &cmd, 1);
            if(cmd == 'C' && buff_pos > ii->word_start) {
                ConstStr new_hint = next_cmdhint(ii->cmdhint, buff+ii->word_start);
                set_hint(new_hint, ii);
            } else if(cmd == 'D' && buff_pos > ii->word_start) {
                ConstStr new_hint = prev_cmdhint(ii->cmdhint);
                set_hint(new_hint, ii);
            }
        } else if (cmd == ESC) { // Double ESC
            term_endline();
            ii->keymap = NULL;
            ii->else_handler = NULL;
        }
    }

    return RRS_NOP;
}

int pk_bs(int fd, InputInterface *ii) {
    UNUSED(fd);
    if(buff_pos > ii->word_start) {
        term_backspace();
        buff_pos--;
        return RRS_NOP;
    } else {
        return RRS_BACKSPACE;
    }
}

int pkac_bs(int fd, InputInterface *ii) {
    UNUSED(fd);
    if(buff_pos > ii->word_start) {
        write(STDOUT_FILENO, "\b", 1);
        buff[--buff_pos] = '\0';

        if(buff_pos == ii->word_start) {
            term_endline();
        }
        else {
            Hash hash = hashofstr(ii->cmdhint->current_hint);
            ConstStr s;
            while(s = next_cmdhint(ii->cmdhint, buff+ii->word_start),
                  s.str != NULL && hashofstr(s) != hash);
        }
            

        return RRS_NOP;
    } else {
        if(ii->word_start > ii->arg_start) {
            ii->word_start -= 2;
            while(ii->word_start > ii->arg_start
                    && buff[ii->word_start != '/'])
                ii->word_start--;
            if(ii->word_start == ii->arg_start) {
                ii->cmdhint->ht_flags = HT_CMD | HT_DIR;
            }
            buff[--buff_pos] = '\0';
            termcur_hmove(-1);
            return RRS_NOP;
        } else {
            ii->cmdhint->current_hint = nostr;
            return RRS_BACKSPACE;
        }
    }
}

int pk_space(int fd, InputInterface *ii) {
    UNUSED(fd);
    UNUSED(ii);
    if(buff_pos > ii->arg_start) {
        write(STDOUT_FILENO, " ", 1);
        return RRS_EOW;
    }

    return RRS_NOP;
}

size_t autocomplete_buff(ConstStr str, int word_start) {
    size_t rem = str.len - (buff_pos - word_start);
    strcpy(buff + word_start, str.str);
    buff_pos += rem;

    return rem;
}

int pkac_space(int fd, InputInterface *ii) {
    UNUSED(fd);
    if(buff_pos > ii->arg_start) {
        if(ii->cmdhint->current_hint.str != NULL) {
            size_t rem = autocomplete_buff(ii->cmdhint->current_hint, ii->word_start);

            if(rem > 0) {
                termcur_hmove(rem+1);
            } else
                write(STDOUT_FILENO, " ", 1);
        } else 
                write(STDOUT_FILENO, " ", 1);

        return RRS_EOW;
    }

    return RRS_NOP;
}

int pkac_tab(int fd, InputInterface *ii) {
    UNUSED(fd);
    if(buff_pos > ii->word_start && ii->cmdhint->hints->strpos > 0) {
        printf("\n");
        StrList *hs = ii->cmdhint->hints;
        for(size_t i = 0; i < hs->strpos; i++) {
            printf("%s ", hs->strbuff[i].str);
        }
        printf("\n");

        writestr(STDOUT_FILENO, ii->promptString);
        if(ii->word_start > ii->arg_start) {
            write(STDOUT_FILENO, buff+ii->arg_start, ii->word_start-ii->arg_start);
        }
        writestr(STDOUT_FILENO, ii->cmdhint->current_hint);
        termcur_hmove((buff_pos - ii->word_start)-ii->cmdhint->current_hint.len);
    }
    return RRS_NOP;   
}

void enter_directory(InputInterface *ii) {
    CmdHint *ch = ii->cmdhint;
    size_t rem = autocomplete_buff(ch->current_hint, ii->word_start);
    termcur_hmove(rem);

    ch->prefix_len = 0;
    ch->current_path = (ConstStr) {
        .str = buff + ii->arg_start,
        .len = buff_pos - ii->arg_start
    };

    closedir(ch->dh);
    ch->dh = NULL;
    ch->dh = opendir(ch->current_path.str);

    if(ch->dh == NULL)
        fprintf(stderr, "Can't open %s\n", ch->current_path.str);

    ii->word_start = buff_pos;
    ii->cmdhint->ht_flags = HT_DIR | HT_EXEC | HT_CUSTOMDIR;
}

int pkac_slash(int fd, InputInterface *ii) {
    UNUSED(fd);

    if(buff_pos == ii->arg_start) {
        buff[buff_pos++] = '/';
        buff[buff_pos] = '\0';
        write(STDOUT_FILENO, "/", 1);

        CmdHint *ch = ii->cmdhint;
        ch->prefix_len = 0;
        ch->current_path = (ConstStr) {
            .str = "/", .len = 1
        };
        closedir(ch->dh);
        ch->dh = NULL;
        ch->dh = opendir(ch->current_path.str);

        if(ch->dh == NULL)
            fprintf(stderr, "Can't open %s\n", ch->current_path.str);

        ii->word_start = buff_pos;
        ii->cmdhint->ht_flags = HT_DIR | HT_EXEC | HT_CUSTOMDIR;

        return RRS_NOP;
    }

    ConstStr chint = ii->cmdhint->current_hint;
    if(chint.len > 0 && chint.str[chint.len-1] == '/') {
        enter_directory(ii);
    }

    return RRS_NOP;
}

int pk_ret(int fd, InputInterface *ii) {
    UNUSED(fd);
    UNUSED(ii);

    if(buff_pos > ii->arg_start) {
        write(STDOUT_FILENO, "\n", 1);
        return RRS_EOL;
    } else {
        return RRS_NOP;
    }
}

int pkac_ret(int fd, InputInterface *ii) {
    UNUSED(fd);

    if(buff_pos > ii->arg_start) {
        if(ii->cmdhint != NULL && ii->cmdhint->current_hint.str != NULL) 
            autocomplete_buff(ii->cmdhint->current_hint, ii->word_start);
        write(STDOUT_FILENO, "\n", 1);
        
        return RRS_EOL;
    } else {
        return RRS_NOP;
    }
}

PromptKey keyset[] = {
    {.key = CTRL_D,       .handler = &pk_eof},
    {.key = ESC,          .handler = &pk_esc},
    {.key = IN_BACKSPACE, .handler = &pk_bs},
    {.key = ' ',          .handler = &pk_space},
    {.key = '\t',         .handler = &pk_space},
    {.key = '\n',         .handler = &pk_ret}
};

PromptKey keyset_ac[] = {
    {.key = ESC,          .handler = &pkac_esc},
    {.key = IN_BACKSPACE, .handler = &pkac_bs},
    {.key = ' ',          .handler = &pkac_space},
    {.key = '\t',         .handler = &pkac_tab},
    {.key = '\n',         .handler = &pkac_ret},
    {.key = '/',          .handler = &pkac_slash}
};

int ii_command_input(char key, int fd, InputInterface *ii) {
    UNUSED(fd);
    buff[buff_pos++] = key;
    buff[buff_pos] = 0;

    ConstStr hint = next_cmdhint(ii->cmdhint, buff + ii->word_start);
    if(hint.str == NULL) {
        buff_pos--;
    } else {
        int cut = buff_pos - ii->word_start - 1;
        int rem = hint.len - cut;
        write(STDOUT_FILENO, hint.str + cut, rem);
        term_endline();
        termcur_hmove(-rem+1);
    }
    return RRS_NOP;
}

void ii_continue_arg(InputInterface *ii) {
    int i;
    for(i = buff_pos;
        i > ii->arg_start && buff[i-1] != '/';
        i--);
    ii->word_start = i;
    if(ii->word_start != ii->arg_start) {
        char str[ii->word_start - ii->arg_start + 1];
        strncpy(str, buff+ii->arg_start, ii->word_start-ii->arg_start);
        str[ii->word_start-ii->arg_start] = 0;
        ii->cmdhint->dh = opendir(str);
        ii->cmdhint->ht_flags =
            (ii->cmdhint->ht_flags | HT_CMD)
                ? HT_DIR | HT_EXEC | HT_CUSTOMDIR
                : HT_DIR | HT_ANYFILE | HT_CUSTOMDIR;
    }
}

#define RRS_IGNORE 0xffff
ReadResult try_keys(PromptKey *keydefs, size_t len, char in, int fd, InputInterface *ii) {
    for(uint i = 0; i < len; i++) {
        if(in == keydefs[i].key) {
            int ret = keydefs[i].handler(fd, ii);
            if(ret == RRS_NOP)
                return (ReadResult) {.status = RRS_NOP};

            else if(ret == RRS_BACKSPACE) {
                return (ReadResult) {.status = ret};
            } else {
                buff[buff_pos++] = 0;
                return (ReadResult) {.status = ret,
                                     .text = buff + ii->arg_start};
            }
        }
    }

    return (ReadResult) {.status = RRS_IGNORE}; 
}

ReadResult read_word(int fd, char *override_arg, InputInterface *ii) {
    int n;
    if(override_arg != NULL) {
        ii->arg_start = override_arg - buff;

        if(ii->continue_arg != NULL)
            ii->continue_arg(ii);
        else
            ii->word_start = ii->arg_start;
    } else {
        ii->arg_start = ii->word_start = buff_pos;
    }

    char sbuff;
    while(BUFF_SIZE - buff_pos >= 8
          && (n = read(fd, &sbuff, 1)) > 0) {

        ReadResult ret = { .status = RRS_IGNORE };
        if(ii->keymap != NULL)
            ret = try_keys(ii->keymap, ii->keymap_len,
                           sbuff, fd, ii);
        if(ret.status == RRS_IGNORE)
            ret = try_keys(keyset, sizeof(keyset)/sizeof(PromptKey),
                           sbuff, fd, ii);

        if(ret.status == RRS_NOP)
            continue;

        if(ret.status != RRS_IGNORE)
            return ret;

        if(ii->else_handler != NULL) {
            int status = ii->else_handler(sbuff, fd, ii);
            if(status == RRS_NOP)
                continue;
            if(status != RRS_IGNORE) {
                return (ReadResult) {
                    .status = status,
                    .text = buff + ii->arg_start
                };
            }
        }

        write(STDOUT_FILENO, &sbuff, 1);
        buff[buff_pos++] = sbuff;
    }

    return (ReadResult) { .status = RRS_OVERFLOW,
                          .text = buff + ii->arg_start };
}

char *args[MAX_ARGS+1];
char ** read_args(int fd, InputInterface *ii) {
    int n = 0;
    buff_pos = 0;
    
    while(n < MAX_ARGS) {
        ii->keymap = keyset_ac;
        ii->else_handler = ii_command_input;
        ii->cmdhint->current_hint.str = NULL;
        ii->cmdhint->dh = NULL;
        ii->cmdhint->ht_flags = n == 0? HT_DIR | HT_CMD : HT_DIR | HT_ANYFILE;

        ReadResult ans = read_word(fd, NULL, ii);
        if(ans.status == RRS_OVERFLOW) {
            fprintf(stderr, "\nOverflow, aborting…\n");
            args[0] = NULL;
            return args;
        }

        if(ans.status == RRS_EOF)
            return NULL;

        while(ans.status == RRS_BACKSPACE) {
            if(n > 0) {
                n--;
                buff_pos--;
                write(STDOUT_FILENO, "\b", 1);
                ans = read_word(fd, args[n], ii);
            } else
                ans = read_word(fd, args[n], ii);
        }

        args[n++] = ans.text;
        
        if(ans.status & RRS_EOL)
            break;
    }

    args[n] = NULL;
    return args;
}

struct termios ito, oto, eto, termopt;
void prompt(const CHLine *handlers, size_t handlers_cnt) {
    PTY ptys[MAX_PTYS];
    for(size_t i = 0; i < MAX_PTYS; i++)
        ptys[i] = NO_PTY;

    char **argv;

    tcgetattr(STDIN_FILENO, &ito);
    tcgetattr(STDOUT_FILENO, &oto);
    tcgetattr(STDERR_FILENO, &eto);

    termopt = ito;
    termopt.c_lflag &= ~(ICANON | ECHO);
    tcsetattr(STDIN_FILENO, TCSAFLUSH, &termopt);

    CmdHint ch = new_cmdhint(handlers, handlers_cnt);
    InputInterface ii = (InputInterface) {
        .keymap = keyset_ac, .keymap_len = sizeof(keyset_ac)/sizeof(PromptKey),
        .cmdhint = &ch, .else_handler = &ii_command_input,
        .continue_arg = &ii_continue_arg,
        .promptString = (ConstStr){ "> ", 2 },
    };


    while(reset_tty(),
          write(STDOUT_FILENO, "> ", 2),
          argv = read_args(STDIN_FILENO, &ii)) {
        if(*argv == NULL) continue;

        CommandHandler ch = NULL;
        for(size_t i = 0; i < handlers_cnt; i++) {
            if(strcmp(args[0], handlers[i].cmd) == 0) {
                ch = handlers[i].handler;
                break;
            }
        }

        if(ch != NULL) {
            ch(args, ptys);
        } else {
            spawn(args, ptys);
        }
    }

    tcsetattr(STDIN_FILENO, TCSAFLUSH, &ito);
    tcsetattr(STDOUT_FILENO, TCSAFLUSH, &oto);
    tcsetattr(STDERR_FILENO, TCSAFLUSH, &eto);
}

void reset_tty() {
    tcsetattr(STDIN_FILENO, TCSAFLUSH, &termopt);
}
