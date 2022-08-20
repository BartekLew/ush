CFLAGS += -g -std=c11 -Wall -Wextra -pedantic -pthread 
 
.PHONY: all clean

all: ush

ush: ush.c

clean:
	rm ush
