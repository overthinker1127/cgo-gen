#ifndef CGOWRAP_B_WRAPPER_H
#define CGOWRAP_B_WRAPPER_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <stdlib.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct BHandle BHandle;

BHandle* cgowrap_B_new(void);
void cgowrap_B_delete(BHandle* self);
int cgowrap_B_Value(const BHandle* self);
void cgowrap_B_SetValue(BHandle* self, int value);
#ifdef __cplusplus
}
#endif

#endif /* CGOWRAP_B_WRAPPER_H */
