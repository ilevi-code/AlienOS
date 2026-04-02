#pragma once

#define NULL ((void*)0)

#define OFFSETOF(t, f) ((uint)(&(((t*)0)->f)))

#define MAX(a, b) ((a) > (b) ? (a) : (b))
#define MIN(a, b) ((a) < (b) ? (a) : (b))
#define CEIL_DIV(val, align) (((val) + (align) - 1) / (align))
#define ARRAY_SIZE(arr) (sizeof(arr) / sizeof(*arr))
