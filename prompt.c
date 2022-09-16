#include "prompt.h"

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

typedef struct {
    char *text;
    int  status;
} ReadResult;

ReadResult read_word(int fd, char *override_arg) {
    int n;
    int line_start = buff_pos;
    if(override_arg != NULL) {
        line_start = override_arg - buff;
    }

    char sbuff;
    while(BUFF_SIZE - buff_pos >= 8
          && (n = read(fd, &sbuff, 1)) > 0) {

        if(sbuff == CTRL_D)
            return (ReadResult) { .status = RRS_EOF };

        if(sbuff == 0x1b) { // ESC
            char cmd;
            if(read(fd, &cmd, 1) == 1) { 
                if(cmd == 0x5b) { // Move Keys
                    read(fd, &cmd, 1);
                } 
            }
        }
        else if(sbuff == IN_BACKSPACE) {
            if(buff_pos > line_start) {
                write(STDOUT_FILENO, "\b", 1);
                buff_pos--;
            } else {
                return (ReadResult) { .status = RRS_BACKSPACE };
            }
        }
        else if(sbuff == ' ' || sbuff == '\t') {
            if(buff_pos > line_start) {
                write(STDOUT_FILENO, " ", 1);
                buff[buff_pos++] = 0;
                return (ReadResult) { .status = 0,
                                      .text = buff + line_start };
            }
        }
        else if(sbuff == '\n') {
            write(STDOUT_FILENO, "\n", 1);
            buff[buff_pos++] = 0;
            return (ReadResult) { .status = RRS_EOL,
                                  .text = buff + line_start };
        } else {
            write(STDOUT_FILENO, &sbuff, 1);
            buff[buff_pos++] = sbuff;
        }
    }

    return (ReadResult) { .status = RRS_OVERFLOW,
                          .text = buff + line_start };
}

char *args[MAX_ARGS+1];
char ** read_args(int fd) {
    int n = 0;
    buff_pos = 0;
    while(n < MAX_ARGS) {
        ReadResult ans = read_word(fd, NULL);
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
                ans = read_word(fd, args[n]);
            } 
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

    while(reset_tty(),
          write(STDOUT_FILENO, "> ", 2),
          argv = read_args(STDIN_FILENO)) {
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
