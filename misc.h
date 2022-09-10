#ifndef __H_MISC
#define __H_MISC 1

#define _XOPEN_SOURCE 600
#include <stdbool.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <errno.h>
#include <signal.h>
#include <ctype.h>

#define UNUSED(x) (void)x

#define CTRL_D 0x04
#define CTRL_X 0x18
#define ERRNO_SIGCAUGHT 0x04

typedef unsigned int uint;

#define BUFF_SIZE 1024
#define MAX_ARGS 1024

#endif
