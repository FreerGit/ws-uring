
CC = gcc
ifeq ($(BUILD), release)
CFLAGS = -std=c2x -O3 -finline-functions -s -DNDEBUG 
else
BUILD = debug
CFLAGS = -std=c2x -Wextra -Wall -Wfloat-equal -Wundef -Wshadow -Wpointer-arith -Wcast-align -Wstrict-prototypes -Wstrict-overflow=5 -Wwrite-strings  -Wcast-qual -Wswitch-default -Wswitch-enum -Wconversion -Wunreachable-code -fno-omit-frame-pointer -fno-var-tracking-assignments -Wformat=2 -Wno-discarded-qualifiers
SANFLAGS = -fsanitize=address -fsanitize=undefined -fsanitize-address-use-after-scope
endif
TARGET := a.out
BUILD_FOLDER = build
INCLUDE = ./stx

CFLAGS += -I$(INCLUDE)

SRCS := $(wildcard test/*.c)
OBJS := $(patsubst %.c,%.o,$(SRCS))

all: $(TARGET)
$(TARGET): $(OBJS)
	$(info BUILD=$(BUILD))
	@$(CC) $(CFLAGS) $(SANFLAGS) -o $@ $(BUILD_FOLDER)/$(notdir $^)  -lrt
%.o: %.c
	@$(CC) $(CFLAGS) $(SANFLAGS)  -c $< -o $(BUILD_FOLDER)/$(notdir $@) -lrt

test: $(TARGET)
	@echo " "
	@./$(TARGET)

clean:
	rm -rf $(TARGET) *.o
	
.PHONY: all clean test

