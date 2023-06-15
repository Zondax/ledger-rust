#pragma once

#include <stdarg.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

#define KEY_SIZE (63 + 1)

#define MESSAGE_SIZE (4095 + 1)

#define MAX_ITEMS 4

typedef struct Item {
  uint8_t title[KEY_SIZE];
  uint8_t message[MESSAGE_SIZE];
} Item;

typedef struct StaxBackend {
  struct Item items[MAX_ITEMS];
  uintptr_t items_len;
  uintptr_t viewable_size;
  bool expert_mode;
} StaxBackend;

extern struct StaxBackend BACKEND_LAZY;
