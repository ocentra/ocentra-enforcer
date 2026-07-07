include config.mk

CC = gcc

all: widget

widget: main.o
	$(CC) -o widget main.o

main.o: main.c
	$(CC) -c main.c

.PHONY: clean
clean:
	rm -f *.o widget
