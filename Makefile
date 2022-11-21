CFLAGS += -g -std=c11 -Wall -Wextra -fstack-protector -pthread
LDFLAGS += -lpthread
 
.PHONY: all clean

all: ush

ush: fds.o ush.o prompt.o cmdhint.o misc.o term.o buffer.o

clean:
	rm ush ush.o fds.o prompt.o misc.o term.o buffer.o cmdhint.o
