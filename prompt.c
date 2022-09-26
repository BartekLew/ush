#include "prompt.h"
#include "cmdhint.h"

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
    int (*handler)(int fd, int line_start, InputInterface *ii);
} PromptKey;

struct ii {
    PromptKey *keymap;
    size_t    keymap_len;
    int       (*else_handler) (char key, int fd, int line_start, InputInterface *ii);
    CmdHint   *cmdhint;
};

int pk_eof(int fd, int line_start, InputInterface *ii) {
    UNUSED(fd);
    UNUSED(line_start);
    UNUSED(ii);
    return RRS_EOF;
}

int pk_esc(int fd, int line_start, InputInterface *ii) {
    UNUSED(line_start);
    UNUSED(ii);
    char cmd;
    if(read(fd, &cmd, 1) == 1) { 
        if(cmd == 0x5b) { // Move Keys
            read(fd, &cmd, 1);
        } 
    }

    return RRS_NOP;
}

int pkac_esc(int fd, int line_start, InputInterface *ii) {
    UNUSED(line_start);
    UNUSED(ii);
    char cmd;
    if(read(fd, &cmd, 1) == 1) { 
        if(cmd == 0x5b) { // Move Keys
            read(fd, &cmd, 1);
            if(cmd == 'C' && buff_pos > line_start) {
                const char *new_hint = next_cmdhint(ii->cmdhint, buff+line_start);
                if(new_hint != NULL) {
                    size_t pos = buff_pos - line_start;
                    strncpy(buff + line_start, new_hint, pos);
                    size_t hintlen = strlen(new_hint);

                    char seq[10];

                    // Move cursor back to beginning of command
                    int n = sprintf(seq, "\x1b[%luD", ii->cmdhint->prefix_len);
                    write(STDOUT_FILENO, seq, n);
                    write(STDOUT_FILENO, new_hint, hintlen);
                    write(STDOUT_FILENO, "\x1b[K", 3);
                    if(hintlen > pos) {
                        n = sprintf(seq, "\x1b[%luD", hintlen - pos);
                        write(STDOUT_FILENO, seq, n);
                    }
                }
            }
        } 
    }

    return RRS_NOP;
}

int pk_bs(int fd, int line_start, InputInterface *ii) {
    UNUSED(fd);
    UNUSED(ii);
    if(buff_pos > line_start) {
        // ^[[P - delete 1 char
        write(STDOUT_FILENO, "\b\x1b[P", 4);
        buff_pos--;
        return RRS_NOP;
    } else {
        return RRS_BACKSPACE;
    }
}

int pkac_bs(int fd, int line_start, InputInterface *ii) {
    UNUSED(fd);
    if(buff_pos > line_start) {
        write(STDOUT_FILENO, "\b", 1);
        buff_pos--;

        // Clear line if last letter removed
        if(buff_pos == line_start)
            write(STDOUT_FILENO, "\x1b[K", 3);

        return RRS_NOP;
    } else {
        ii->cmdhint->current_hint = NULL;
        return RRS_BACKSPACE;
    }
}

int pk_space(int fd, int line_start, InputInterface *ii) {
    UNUSED(fd);
    UNUSED(ii);
    if(buff_pos > line_start) {
        write(STDOUT_FILENO, " ", 1);
        return RRS_EOW;
    }

    return RRS_NOP;
}

size_t autocomplete_buff(const char *str, int line_start) {
    size_t len = strlen(str);
    size_t rem = len - (buff_pos - line_start);
    strcpy(buff + line_start, str);
    buff_pos += rem;

    return rem;
}

int pkac_space(int fd, int line_start, InputInterface *ii) {
    UNUSED(fd);
    if(buff_pos > line_start && ii->cmdhint->current_hint != NULL) {
        size_t rem = autocomplete_buff(ii->cmdhint->current_hint, line_start);

        char seq[10];
        if(rem > 0) {
            // move cursor to the end of existing autocompleted
            // string and add a space there.
            int n = sprintf(seq, "\x1b[%luC ", rem);
            write(STDOUT_FILENO, seq, n);
        } else
            write(STDOUT_FILENO, " ", 1);

        return RRS_EOW;
    }

    return RRS_NOP;
}

int pk_ret(int fd, int line_start, InputInterface *ii) {
    UNUSED(fd);
    UNUSED(line_start);
    UNUSED(ii);

    if(buff_pos > line_start) {
        write(STDOUT_FILENO, "\n", 1);
        return RRS_EOL;
    } else {
        return RRS_NOP;
    }
}

int pkac_ret(int fd, int line_start, InputInterface *ii) {
    UNUSED(fd);

    if(buff_pos > line_start && ii->cmdhint != NULL && ii->cmdhint->current_hint != NULL) {
        autocomplete_buff(ii->cmdhint->current_hint, line_start);
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
    {.key = '\t',         .handler = &pkac_space},
    {.key = '\n',         .handler = &pkac_ret}
};

int ii_command_input(char key, int fd, int line_start, InputInterface *ii) {
    UNUSED(fd);
    buff[buff_pos++] = key;
    buff[buff_pos] = 0;

    const char *hint = next_cmdhint(ii->cmdhint, buff + line_start);
    if(hint == NULL) {
        buff_pos--;
    } else {
        int cut = buff_pos - line_start - 1;
        int rem = strlen(hint) - cut;
        write(STDOUT_FILENO, hint + cut, rem);

        char seq[10];
        // ^[[K - erase to the end of line
        // ^[[nD move n chars left
        if(rem > 1) {
            int n = sprintf(seq, "\x1b[K\x1b[%dD", rem-1);
            write(STDOUT_FILENO, seq, n);
        }
    }
    return RRS_NOP;
}

#define RRS_IGNORE 0xffff
ReadResult try_keys(PromptKey *keydefs, size_t len, char in, int fd, int line_start, InputInterface *ii) {
    for(uint i = 0; i < len; i++) {
        if(in == keydefs[i].key) {
            int ret = keydefs[i].handler(fd, line_start, ii);
            if(ret == RRS_NOP)
                return (ReadResult) {.status = RRS_NOP};

            else if(ret == RRS_BACKSPACE) {
                return (ReadResult) {.status = ret};
            } else {
                buff[buff_pos++] = 0;
                return (ReadResult) {.status = ret,
                                     .text = buff + line_start};
            }
        }
    }

    return (ReadResult) {.status = RRS_IGNORE}; 
}

ReadResult read_word(int fd, char *override_arg, InputInterface *ii) {
    int n;
    UNUSED(ii);
    int line_start = buff_pos;
    if(override_arg != NULL) {
        line_start = override_arg - buff;
    }

    char sbuff;
    while(BUFF_SIZE - buff_pos >= 8
          && (n = read(fd, &sbuff, 1)) > 0) {

        ReadResult ret = { .status = RRS_IGNORE };
        if(ii != NULL && ii->keymap != NULL)
            ret = try_keys(ii->keymap, ii->keymap_len,
                           sbuff, fd, line_start, ii);
        if(ret.status == RRS_IGNORE)
            ret = try_keys(keyset, sizeof(keyset)/sizeof(PromptKey),
                           sbuff, fd, line_start, ii);

        if(ret.status == RRS_NOP)
            continue;

        if(ret.status != RRS_IGNORE)
            return ret;

        if(ii != NULL && ii->else_handler != NULL) {
            int status = ii->else_handler(sbuff, fd, line_start, ii);
            if(status == RRS_NOP)
                continue;
            if(status != RRS_IGNORE) {
                return (ReadResult) {
                    .status = status,
                    .text = buff + line_start
                };
            }
        }

        write(STDOUT_FILENO, &sbuff, 1);
        buff[buff_pos++] = sbuff;
    }

    return (ReadResult) { .status = RRS_OVERFLOW,
                          .text = buff + line_start };
}

char *args[MAX_ARGS+1];
char ** read_args(int fd, CmdHint *ch) {
    UNUSED(ch);
    int n = 0;
    buff_pos = 0;
    
    InputInterface ii = (InputInterface) {
        .keymap = keyset_ac, .keymap_len = sizeof(keyset_ac)/sizeof(PromptKey),
        .cmdhint = ch, .else_handler = &ii_command_input
    };

    while(n < MAX_ARGS) {
        ReadResult ans = read_word(fd, NULL, (n == 0?&ii:NULL));
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
                ans = read_word(fd, args[n], (n == 0?&ii:NULL));
            } else
                ans = read_word(fd, args[n], (n == 0?&ii:NULL));
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

    while(reset_tty(),
          write(STDOUT_FILENO, "> ", 2),
          argv = read_args(STDIN_FILENO, &ch)) {
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
