CFLAGS += -g -std=c99 -Wall -Wextra -pedantic

.PHONY: all clean

all: ush

ush: ush.c

clean:
	rm ush
